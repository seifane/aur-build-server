use std::error::Error;
use std::{env, fs};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};
use relative_path::RelativePath;
use simple_error::SimpleError;
use clap::Parser;

use crate::args::Args;
use crate::errors::package_build_error::PackageBuildError;
use crate::package_manager::Package;
use crate::utils::git::clone_repo;
use crate::utils::log::write_logs;
use crate::utils::pkgbuild::{parse_opt_deps, read_dependencies};

pub fn copy_package_to_repo(repo_name: String) -> Result<(), Box<dyn Error>>{
    println!("Copying packages for {}", repo_name);

    let serve_path = Path::new("serve");
    if !serve_path.exists() {
        fs::create_dir(serve_path).unwrap();
    }

    let repo_data_str = format!("data/{}", repo_name).to_string();
    let repo_data_path = Path::new(repo_data_str.as_str());

    for file in fs::read_dir(repo_data_path)? {
        let file_res = file?;
        if file_res.file_name().to_str().unwrap().contains(".pkg.tar.zst") {
            let new_path = format!(
                "serve/{}",
                file_res.file_name().to_str().ok_or(SimpleError::new("Failed to format path"))?
            );
            let move_path = RelativePath::new(new_path.as_str());

            fs::rename(file_res.path(), move_path.to_path(env::current_dir()?))?;
        }
    }

    Ok(())
}

pub fn build_package(package: &Package) -> std::result::Result<(), PackageBuildError> {
    let args: Args = Args::parse();

    let mut cmd_args: String = String::new();

    if args.sign {
        cmd_args += " --sign";
    }

    if package.run_before.is_some() {
        let pre_run_output = Command::new("sh")
            .arg("-c")
            .arg(package.run_before.as_ref().unwrap()).output();

        let out = pre_run_output.unwrap();

        write_logs(package.name.as_str(), out.stdout.as_slice(), "stdout_before").unwrap_or(());
        write_logs(package.name.as_str(), out.stderr.as_slice(), "stderr_before").unwrap_or(());

        let status_code = out.status;
        if status_code.code().unwrap() != 0 {
            println!("Failed run_before for {} with code {}", package.name, status_code.code().unwrap());
            return Err(PackageBuildError::new(status_code));
        }
    }

    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("cd data/{}; makepkg --syncdeps --clean --noconfirm{}", package.name, cmd_args)).output();

    let out = output.unwrap();

    write_logs(package.name.as_str(), out.stdout.as_slice(), "stdout").unwrap_or(());
    write_logs(package.name.as_str(), out.stderr.as_slice(), "stderr").unwrap_or(());

    let status_code = out.status;
    if status_code.code().unwrap() != 0 {
        println!("Failed makepkg for {} with code {}", package.name, status_code.code().unwrap());
        return Err(PackageBuildError::new(status_code));
    }

    Ok(())
}

pub fn install_dependencies(package: &Package, dependency_lock: Arc<(Mutex<bool>, Condvar)>) {
    let &(ref lock, ref cvar) = &*dependency_lock;

    {
        let mut guard = lock.lock().unwrap();
        println!("Locked");
        while *guard {
            println!("Waiting for lock to install dependencies");
            guard = cvar.wait(guard).unwrap();
        }
        *guard = true;
    }

    let mut deps = read_dependencies(package, "depends").unwrap();
    deps.extend(read_dependencies(package, "makedepends").unwrap());
    deps.extend(read_dependencies(package, "checkdepends").unwrap());
    deps.extend(parse_opt_deps(
        read_dependencies(package, "optdepends").unwrap()
    ));

    println!("Getting dependencies : {}", deps.join(", "));

    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("sudo pacman -Sy --noconfirm {}", deps.join(" "))).output().unwrap();
    println!("{} {}", String::from_utf8(output.stdout).unwrap(), String::from_utf8(output.stderr).unwrap());

    *lock.lock().unwrap() = false;
    cvar.notify_one();
}

pub fn make_package(package: &Package, dependency_lock: Arc<(Mutex<bool>, Condvar)>, force: bool) -> Result<(), Box<dyn std::error::Error>>  {
    println!("Cloning {} ...", package.name);

    let changed = clone_repo(package.name.as_str())?;
    if changed || force {
        install_dependencies(package, dependency_lock);
        println!("Building {} ...", package.name);
        build_package(package)?;
    }
    Ok(())
}