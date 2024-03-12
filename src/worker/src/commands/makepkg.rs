use async_recursion::async_recursion;
use log::{debug, error, info};
use srcinfo::Srcinfo;
use tokio::process::Command;
use common::models::Package;
use crate::commands::CommandResult;
use crate::errors::PackageBuildError;
use crate::commands::git::clone_repo;
use crate::commands::pacman::is_package_in_repo;
use crate::logs::{LogSection, write_log_section};
use crate::models::{PackageBuild};
use crate::utils::sanitize_dependency;


async fn handle_run_before(package: &Package) -> Result<(), PackageBuildError>{
    if let Some(run_before) = package.run_before.as_ref() {
        info!("Running run_before for {}", package.name);
        let pre_run_output = Command::new("sh")
            .arg("-c")
            .arg(run_before).output().await;

        let out = pre_run_output.unwrap();

        write_log_section(package.name.as_str(), LogSection::RunBeforeOut, out.stdout.as_slice()).await.unwrap();
        write_log_section(package.name.as_str(), LogSection::RunBeforeErr, out.stderr.as_slice()).await.unwrap();

        let status_code = out.status;
        if status_code.code().unwrap_or(-1) != 0 {
            let message = format!("Failed run_before for {} with code {}", package.name, status_code.code().unwrap_or(-1));
            error!("{message}");
            return Err(PackageBuildError::new(message, Some(status_code)));
        }
    }
    Ok(())
}

async fn build_makepkg(package_name: &String, is_dependency: bool) -> Result<CommandResult, PackageBuildError> {
    info!("Running makepkg for {}", package_name);

    let mut cmd = Command::new("makepkg");
    cmd.current_dir(format!("data/{}", package_name))
        .arg("--syncdeps")
        .arg("--clean")
        .arg("--noconfirm");

    if is_dependency {
        cmd.arg("--install").arg("--asdeps");
    }

    let out = cmd.output().await.map_err(|e| {
        PackageBuildError::new(format!("Error getting output of makepkg command {}", e), None)
    })?;

    Ok(CommandResult::from_output(out))
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

    debug!("Found AUR dependencies for {} {:?}", srcinfo.base.pkgbase, deps);

    deps.dedup();
    deps
}

#[async_recursion]
pub async fn handle_aur_deps(package: &Package, deps: Vec<String>) -> Result<Vec<String>, PackageBuildError>
{
    let mut added_deps = Vec::new();
    for dep in deps.iter() {
        info!("Installing AUR dep {}", dep);

        clone_repo(dep)?;
        let src_info = get_src_info(dep).await.map_err(|e| {
            PackageBuildError::new(format!("Failed to get src info {}", e.to_string()), None)
        })?;

        let deps_of_dep = extract_aur_deps(&src_info).await;
        let mut installed = handle_aur_deps(package, deps_of_dep).await?;

        let result = build_makepkg(&dep, true).await?;

        write_log_section(package.name.as_str(), LogSection::DepsOut(dep.clone()), result.stdout.as_bytes()).await.unwrap();
        write_log_section(package.name.as_str(), LogSection::DepsErr(dep.clone()), result.stderr.as_bytes()).await.unwrap();

        if !result.success() {
            error!("Failed makepkg for {} AUR dependency for {} with code {}, check build logs for full output", dep, package.name, result.status.code().unwrap_or(-1));
            return Err(PackageBuildError::new(String::from("Failed makepkg"), Some(result.status)));
        }

        added_deps.append(&mut installed);
        added_deps.push(dep.clone());
    }

    Ok(added_deps)
}

pub async fn make_package(package: &Package) -> Result<PackageBuild, PackageBuildError> {

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

    let installed_deps = handle_aur_deps(package, extract_aur_deps(&src_info).await).await?;
    info!("Installed additional deps {:?}", installed_deps);

    let result = build_makepkg(&package.name, false).await?;
    write_log_section(package.name.as_str(), LogSection::MakePkgOut, result.stdout.as_bytes()).await.unwrap();
    write_log_section(package.name.as_str(), LogSection::MakePkgErr, result.stderr.as_bytes()).await.unwrap();

    Ok(PackageBuild::new(true, version, installed_deps))
}