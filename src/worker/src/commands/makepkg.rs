use async_recursion::async_recursion;
use log::{error, info};
use srcinfo::Srcinfo;
use tokio::process::Command;
use crate::errors::PackageBuildError;
use crate::commands::git::clone_repo;
use crate::commands::pacman::is_package_in_repo;
use crate::commands::write_logs;
use crate::models::{Package, PackageBuild};
use crate::utils::sanitize_dependency;


async fn handle_run_before(package: &Package) -> Result<(), PackageBuildError>{
    if package.run_before.is_some() {
        info!("Running run_before for {}", package.name);
        let pre_run_output = Command::new("sh")
            .arg("-c")
            .arg(package.run_before.as_ref().unwrap()).output().await;

        let out = pre_run_output.unwrap();

        write_logs(package.name.as_str(), out.stdout.as_slice(), "stdout_run_before").unwrap_or(());
        write_logs(package.name.as_str(), out.stderr.as_slice(), "stderr_run_before").unwrap_or(());

        let status_code = out.status;
        if status_code.code().unwrap() != 0 {
            error!("Failed run_before for {} with code {}", package.name, status_code.code().unwrap());
            return Err(PackageBuildError::new(String::from("Failed run before"),Some(status_code)));
        }
    }
    Ok(())
}

async fn build_makepkg(package_name: &String, install: bool) -> Result<(), PackageBuildError> {
    info!("Running makepkg for {}", package_name);

    let mut cmd = Command::new("makepkg");
    cmd.current_dir(format!("data/{}", package_name))
        .arg("--syncdeps")
        .arg("--clean")
        .arg("--noconfirm");

    if install {
        cmd.arg("--install").arg("--asdeps");
    }

    let out = cmd.output().await.map_err(|e| {
        PackageBuildError::new(format!("Error getting output of makepkg command {}", e), None)
    })?;

    write_logs(package_name.as_str(), out.stdout.as_slice(), "stdout").unwrap_or(());
    write_logs(package_name.as_str(), out.stderr.as_slice(), "stderr").unwrap_or(());

    if out.status.code().is_none() || out.status.code().unwrap() != 0 {
        error!("Failed makepkg for {} with code {}", package_name, out.status.code().unwrap_or(-1));
        return Err(PackageBuildError::new(String::from("Failed makepkg"),Some(out.status)));
    }

    Ok(())
}

pub async fn get_src_info(package_name: &String) -> Result<Srcinfo, PackageBuildError>
{
    let output = Command::new("makepkg").arg("--printsrcinfo")
        .current_dir(format!("data/{}", package_name))
        .output().await.map_err(|e| {
        PackageBuildError::new(format!("Failed to execute makepkg printsrcinfo for {}, {}", package_name, e), None)
    })?;

    Srcinfo::parse_buf(output.stdout.as_slice()).map_err(|e| {
        PackageBuildError::new(format!("Failed to parse srcinfo {}, {}", package_name, e), None)
    })
}

pub async fn extract_aur_deps(srcinfo: &Srcinfo) -> Vec<String>
{
    let mut deps = Vec::new();

    for pkg in srcinfo.pkgs.iter() {
        for ds in pkg.depends.iter().map(|i| i.vec.clone()) {
            for d in ds.iter() {
                let sanitized = sanitize_dependency(d.as_str());
                if !is_package_in_repo(&sanitized).await {
                    deps.push(sanitized);
                }
            }
        }
    }

    deps.dedup();
    deps
}

#[async_recursion]
pub async fn handle_aur_deps(deps: Vec<String>) -> Result<Vec<String>, PackageBuildError>
{
    let mut added_deps = Vec::new();
    for dep in deps.iter() {
        info!("Installing AUR dep {}", dep);

        clone_repo(dep)?;
        let src_info = get_src_info(dep).await.map_err(|e| {
            PackageBuildError::new(format!("Failed to get src info {}", e.to_string()), None)
        })?;

        let deps_of_dep = extract_aur_deps(&src_info).await;
        let mut installed = handle_aur_deps(deps_of_dep).await?;

        build_makepkg(&dep, true).await?;

        added_deps.append(&mut installed);
        added_deps.push(dep.clone());
    }

    Ok(added_deps)
}

pub async fn make_package(package: &Package) -> Result<PackageBuild, PackageBuildError> {
    clone_repo(&package.name)?;

    let src_info = get_src_info(&package.name).await.map_err(|e| {
        PackageBuildError::new(format!("Failed to get src info {}", e.to_string()), None)
    })?;

    let mut version = src_info.base.pkgver.clone();
    version += src_info.base.pkgrel.as_str();
    version += src_info.base.epoch.clone().unwrap_or("".to_string()).as_str();

    if let Some(last_built_version) = &package.last_built_version {
            if last_built_version == &version {
                info!("Found same version for package, skipping build ...");
                return Ok(PackageBuild::new(false, version, Vec::new()))
            }
    }

    handle_run_before(&package).await?;

    let installed_deps = handle_aur_deps(extract_aur_deps(&src_info).await).await?;

    info!("Installed additional deps {:?}", installed_deps);

    build_makepkg(&package.name, false).await?;

    Ok(PackageBuild::new(true, version, installed_deps))
}