use std::error::Error;
use std::{env, fs};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};
use relative_path::RelativePath;
use simple_error::SimpleError;

use crate::errors::package_build_error::PackageBuildError;
use crate::utils::git::clone_repo;
use crate::utils::log::write_logs;
use crate::utils::{makepkg, pacman};
use crate::utils::package_data::Package;

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

    makepkg::build(&package.name, false)?;

    Ok(())
}

pub fn install_dependencies(package: &Package, dependency_lock: Arc<(Mutex<bool>, Condvar)>) -> Result<(), Box<dyn Error>> {
    let &(ref lock, ref cvar) = &*dependency_lock;

    let deps = makepkg::get_dependencies(&package.name);
    let (repo_deps, _) = pacman::split_repo_aur_packages(deps);

    if repo_deps.len() > 0 {
        {
            let mut guard = lock.lock().unwrap();
            while *guard {
                debug!("Waiting for lock to install dependencies");
                guard = cvar.wait(guard).unwrap();
            }
            *guard = true;
        }

        info!("Installing dependencies from repos: {}", repo_deps.join(", "));

        let output = Command::new("sh")
            .arg("-c")
            .arg(format!("sudo pacman -Syu --noconfirm {}", repo_deps.join(" "))).output()?;

        write_logs(package.name.as_str(), output.stdout.as_slice(), "stdout_deps").unwrap_or(());
        write_logs(package.name.as_str(), output.stderr.as_slice(), "stderr_deps").unwrap_or(());
    }

    *lock.lock().unwrap() = false;
    cvar.notify_one();
    Ok(())
}

pub fn make_package(package: &mut Package, dependency_lock: Arc<(Mutex<bool>, Condvar)>) -> Result<(), Box<dyn Error>>  {
    info!("Cloning {} ...", package.name);

    let commit_id = clone_repo(&package.name)?;

    info!("Building {} ...", package.name);

    build_package(package, dependency_lock)?;
    package.last_build_commit = Some(commit_id);
    Ok(())
}