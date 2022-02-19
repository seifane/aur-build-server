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
use crate::get_package_data;
use crate::utils::git::clone_repo;
use crate::utils::log::write_logs;
use crate::utils::package_data::Package;
use crate::utils::pkgbuild::{parse_opt_deps, read_dependencies};

pub fn copy_package_to_repo(package_name: &String) -> Result<(), Box<dyn Error>>{
    debug!("Copying packages for {}", package_name);

    let serve_path = Path::new("serve");
    if !serve_path.exists() {
        fs::create_dir(serve_path).unwrap();
    }

    let repo_data_str = format!("data/{}", package_name).to_string();
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

pub fn run_makepkg(package_name: &String, install: bool) -> Result<(), PackageBuildError> {
    let args: Args = Args::parse();

    let mut cmd_args: String = String::new();

    if args.sign {
        cmd_args += " --sign";
    }

    if install {
        cmd_args += " --install";
    }

    debug!("Running makepkg for {}", package_name);

    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("cd data/{}; makepkg --syncdeps --clean --noconfirm{}", package_name, cmd_args)).output();

    let out = output.unwrap();

    write_logs(package_name.as_str(), out.stdout.as_slice(), "stdout").unwrap_or(());
    write_logs(package_name.as_str(), out.stderr.as_slice(), "stderr").unwrap_or(());

    let status_code = out.status;
    if status_code.code().unwrap() != 0 {
        error!("Failed makepkg for {} with code {}", package_name, status_code.code().unwrap());
        return Err(PackageBuildError::new(status_code));
    }
    debug!("Ok makepkg for {}", package_name);

    Ok(())
}

pub fn build_package(package: &Package, dependency_lock: Arc<(Mutex<bool>, Condvar)>) -> std::result::Result<(), PackageBuildError> {
    install_dependencies(package, dependency_lock).unwrap();
    if package.run_before.is_some() {
        debug!("Running run_before for {}", package.name);
        let pre_run_output = Command::new("sh")
            .arg("-c")
            .arg(package.run_before.as_ref().unwrap()).output();

        let out = pre_run_output.unwrap();

        write_logs(package.name.as_str(), out.stdout.as_slice(), "stdout_before").unwrap_or(());
        write_logs(package.name.as_str(), out.stderr.as_slice(), "stderr_before").unwrap_or(());

        let status_code = out.status;
        if status_code.code().unwrap() != 0 {
            error!("Failed run_before for {} with code {}", package.name, status_code.code().unwrap());
            return Err(PackageBuildError::new(status_code));
        }
    }

    run_makepkg(&package.name, false)?;

    Ok(())
}

pub fn get_dependencies(package: &Package) -> Vec<String> {
    debug!("Getting dependencies for {}", package.name);
    clone_repo(&package.name).unwrap();
    let mut deps = read_dependencies(package, "depends").unwrap();
    deps.extend(read_dependencies(package, "makedepends").unwrap());
    deps.extend(read_dependencies(package, "checkdepends").unwrap());
    deps.extend(parse_opt_deps(
        read_dependencies(package, "optdepends").unwrap()
    ));
    deps
}

pub fn filter_aur_deps(deps: Vec<String>) -> Vec<String> {
    let mut aur_deps = Vec::new();

    for dep in deps.iter() {
        let res = get_package_data(dep).unwrap();
        if res.result_count == 1 {
            aur_deps.push(dep.clone());
        }
    }

    aur_deps
}

pub fn install_dependencies(package: &Package, dependency_lock: Arc<(Mutex<bool>, Condvar)>) -> Result<(), Box<dyn Error>> {
    let &(ref lock, ref cvar) = &*dependency_lock;

    let mut deps = read_dependencies(package, "depends")?;
    deps.extend(read_dependencies(package, "makedepends")?);
    deps.extend(read_dependencies(package, "checkdepends")?);
    deps.extend(parse_opt_deps(
        read_dependencies(package, "optdepends")?
    ));

    deps.retain(|dep|  {
        let res = get_package_data(dep).unwrap();
        if res.result_count == 1 {
            return false;
        }
        return true;
    });

    if deps.len() > 0 {
        {
            let mut guard = lock.lock().unwrap();
            while *guard {
                debug!("Waiting for lock to install dependencies");
                guard = cvar.wait(guard).unwrap();
            }
            *guard = true;
        }

        debug!("Installing dependencies : {}", deps.join(", "));

        let output = Command::new("sh")
            .arg("-c")
            .arg(format!("sudo pacman -Sy --noconfirm {}", deps.join(" "))).output()?;

        write_logs(package.name.as_str(), output.stdout.as_slice(), "stdout_deps").unwrap_or(());
        write_logs(package.name.as_str(), output.stderr.as_slice(), "stderr_deps").unwrap_or(());
    }

    *lock.lock().unwrap() = false;
    cvar.notify_one();
    Ok(())
}

pub fn make_package(package: &mut Package, dependency_lock: Arc<(Mutex<bool>, Condvar)>, force: bool) -> Result<(), Box<dyn Error>>  {
    info!("Cloning {} ...", package.name);

    let commit_id = clone_repo(&package.name)?;

    if package.last_build_commit.is_some() && package.last_build_commit.clone().unwrap() == commit_id && !force {
        info!("Skipping {}, same commit", package.name);
        return Ok(());
    }

    info!("Building {} ...", package.name);
    build_package(package, dependency_lock)?;
    package.last_build_commit = Some(commit_id);
    Ok(())
}