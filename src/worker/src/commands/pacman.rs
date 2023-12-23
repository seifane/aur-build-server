use std::error::Error;
use log::{info, warn};
use tokio::process::Command;

pub async fn pacman_update_repos() -> Result<(), Box<dyn Error + Send + Sync>>
{
    let output = Command::new("sudo")
        .arg("pacman")
        .arg("-Syy")
        .output().await?;
    if output.status.code().unwrap() != 0 {
        warn!(
                "Error while updating repos, continuing without update ...\nstdout: {:?}\nstderr: {:?}",
                String::from_utf8(output.stdout),
                String::from_utf8(output.stderr)
            );
    };
    Ok(())
}

pub async fn is_package_in_repo(package_name: &String) -> bool {
    let output = Command::new("pacman")
        .arg("-Ss")
        .arg(format!("^{}$", package_name))
        .output()
        .await;

    if output.is_err() {
        warn!("Could not check pacman for {}, {}", package_name, output.unwrap_err());
        return false;
    }
    output.unwrap().status.success()
}

pub async fn _uninstall_package(package_name: &String) -> bool {
    info!("Uninstalling package {}", package_name);
    let output = Command::new("sudo")
        .arg("pacman")
        .arg("-R")
        .arg("--noconfirm")
        .arg(package_name)
        .output()
        .await;

    if output.is_err() {
        warn!("Error while uninstalling {}, {}", package_name, output.unwrap_err());
        return false;
    }
    output.unwrap().status.success()
}

async fn get_orphan_deps() -> Vec<String>
{
    let output = Command::new("pacman")
        .arg("-Qqtd")
        .output()
        .await;

    if let Ok(output) = output {
        let raw_packages = String::from_utf8(output.stdout).unwrap();
        return raw_packages.split('\n').filter(|i| i.len() > 1).map(|i| i.to_string()).collect();
    }

    Vec::new()
}

pub async fn clear_installed_dependencies() -> bool
{
    let orphans = get_orphan_deps().await;
    if orphans.len() == 0 {
        return true;
    }

    info!("Clearing installed packages '{:?}'", orphans);

    let mut command: Command = Command::new("sudo");
    command.arg("pacman")
        .arg("-Rns")
        .arg("--noconfirm");
    for package in orphans.iter() {
        command.arg(package);
    }

    let output = command.output()
        .await;

    if output.is_err() {
        warn!("Error while clearing packages {}", output.unwrap_err());
        return false;
    }
    output.unwrap().status.success()
}