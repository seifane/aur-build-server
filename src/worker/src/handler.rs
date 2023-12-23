use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use log::{info, warn};
use tokio::fs::{create_dir, remove_dir_all};
use tokio::sync::RwLock;
use common::models::WorkerStatus;
use crate::commands::makepkg::make_package;
use crate::commands::pacman::{clear_installed_dependencies, pacman_update_repos};
use crate::worker::Worker;

pub async fn process_package(worker: Arc<RwLock<Worker>>) -> Result<(), Box<dyn Error + Sync + Send>>
{
    let package = worker.read().await.current_package.clone();
    if package.is_none() {
        return Ok(());
    }
    let package = package.unwrap();

    info!("Starting to process package {}", package.name);

    let logs_path = Path::new("worker_logs/");
    if logs_path.exists() {
        remove_dir_all(logs_path).await?;
    }
    create_dir(logs_path).await?;

    worker.write().await.set_state(WorkerStatus::UPDATING)?;
    let pacman_update_res = pacman_update_repos().await;
    if let Err(e) = pacman_update_res {
        warn!("Failed to update pacman {}", e);
    }
    worker.write().await.set_state(WorkerStatus::WORKING)?;

    let build_result = make_package(&package).await;

    let http_client = worker.read().await.get_http_client();
    http_client.lock().await.upload_packages(&package.name, build_result.clone()).await?;

    worker.write().await.set_current_package(None);
    worker.write().await.set_state(WorkerStatus::CLEANING)?;

    if let Ok(package_build) = build_result {
        info!("Built additional aur packages '{:?}'", package_build.additional_packages);
    }

    let _ = remove_dir_all("data").await;
    let _ = create_dir("data").await;
    clear_installed_dependencies().await;

    worker.write().await.set_state(WorkerStatus::STANDBY)?;

    Ok(())
}