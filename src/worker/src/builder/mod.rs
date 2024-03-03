use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use log::{info, warn};
use tokio::fs::{create_dir, remove_dir_all};
use tokio::sync::RwLock;
use common::models::{Package, WorkerStatus};
use crate::commands::git::{apply_patches, clone_repo};
use crate::commands::makepkg::make_package;
use crate::commands::pacman::{clear_installed_dependencies, pacman_update_repos};
use crate::errors::PackageBuildError;
use crate::models::PackageBuild;
use crate::worker::Worker;

pub async fn init_logs() -> Result<(), Box<dyn Error + Send + Sync>>
{
    let logs_path = Path::new("worker_logs/");
    if logs_path.exists() {
        remove_dir_all(logs_path).await?;
    }
    create_dir(logs_path).await?;
    Ok(())
}

pub async fn fetch_package(package: &Package) -> Result<(), Box<dyn Error + Send + Sync>>
{
    let repository = clone_repo(&package.name)?;
    apply_patches(&package, repository).await?;
    Ok(())
}

pub async fn send_job_result(
    worker: Arc<RwLock<Worker>>,
    package: &Package,
    build_result: Result<PackageBuild, PackageBuildError>
) -> Result<(), Box<dyn Error + Sync + Send>>
{
    let http_client = worker.read().await.get_http_client();
    http_client.lock().await.upload_packages(&package.name, build_result.clone()).await?;
    worker.write().await.set_current_package(None);
    Ok(())
}

pub async fn post_build_clean(worker: Arc<RwLock<Worker>>) -> Result<(), Box<dyn Error + Sync + Send>> {
    worker.write().await.set_state(WorkerStatus::CLEANING)?;

    remove_dir_all("data").await?;
    create_dir("data").await?;
    clear_installed_dependencies().await;
    Ok(())
}

pub async fn process_package(worker: Arc<RwLock<Worker>>) -> Result<(), Box<dyn Error + Sync + Send>>
{
    let package = worker.read().await.current_package.clone();
    if package.is_none() {
        return Ok(());
    }
    let package = package.unwrap();
    info!("Starting to process package {}", package.name);

    init_logs().await?;

    worker.write().await.set_state(WorkerStatus::UPDATING)?;
    if let Err(e) = pacman_update_repos().await {
        warn!("Failed to update pacman {}", e);
    }
    worker.write().await.set_state(WorkerStatus::WORKING)?;

    info!("Fetch package");
    let fetch_res = fetch_package(&package).await;

    let build_result = if let Err(e) = fetch_res {
        Err(PackageBuildError::new(format!("Failed to fetch package {:?}", e), None))
    } else {
        info!("Starting build");
        let build_result = make_package(&package).await;
        if let Ok(package_build) = build_result.as_ref() {
            info!("Built additional aur packages '{:?}'", package_build.additional_packages);
        }
        build_result
    };

    send_job_result(worker.clone(), &package, build_result).await?;

    post_build_clean(worker.clone()).await?;

    worker.write().await.set_state(WorkerStatus::STANDBY)?;
    Ok(())
}