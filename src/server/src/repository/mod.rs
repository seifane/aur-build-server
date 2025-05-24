mod manager;

use crate::models::config::Config;
use crate::persistence::package_store::{Package};
use crate::repository::manager::RepositoryManager;
use anyhow::{Result};
use chrono::Utc;
use common::models::PackageStatus;
use log::{debug, error, info, warn};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use actix_multipart::form::tempfile::TempFile;
use tokio::fs;
use tokio::sync::{Mutex, RwLock};

pub struct Repository {
    path: PathBuf,
    build_logs_path: PathBuf,

    manager: Arc<Mutex<RepositoryManager>>,
}

impl Repository {
    pub async fn from_config(config: Arc<RwLock<Config>>) -> Result<Self> {
        let manager = RepositoryManager::from_config(config.read().await.deref()).await?;

        Ok(Repository {
            path: config.read().await.serve_path.clone(),
            build_logs_path: config.read().await.build_logs_path.clone(),

            manager: Arc::new(Mutex::new(manager)),
        })
    }

    pub async fn handle_package_build_output(
        &mut self,
        package: &mut Package,
        version: Option<String>,
        error: Option<String>,
        log_files: Vec<TempFile>,
        files: Vec<TempFile>,
    ) -> Result<()> {
        if !self.build_logs_path.exists() {
            if let Err(e) = tokio::fs::create_dir_all(&self.build_logs_path).await {
                warn!("Unable to create build logs directory: {}", e);
            }
        }

        for log_file in log_files {
            if let Some(filename) = log_file.file_name {
                let dest = &self.build_logs_path.join(&filename);
                debug!("Copying {:?} to {:?}", log_file.file.path(), dest);
                match fs::copy(log_file.file.path(), &self.build_logs_path.join(&filename)).await {
                    Ok(_) => info!("Successfully persisted log file '{}'", filename),
                    Err(e) => error!("Unable to persist log file '{}': '{}'", filename, e),
                };
                fs::remove_file(log_file.file.path()).await?;
            }
        }
        let mut package_files = Vec::new();

        for file in files {
            if let Some(filename) = file.file_name {
                let dest = &self.path.join(&filename);
                debug!("Copying {:?} to {:?}", file.file.path(), dest);
                match fs::copy(file.file.path(), &self.path.join(&filename)).await {
                    Ok(_) => {
                        info!("Successfully persisted package file '{}'", filename);
                        package_files.push(filename)
                    },
                    Err(e) => error!("Unable to persist package file '{}': '{}'", filename, e),
                }
                fs::remove_file(file.file.path()).await?;
            }
        }

        self.update_package_state_from_build_data(
            package,
            package_files.clone(),
            error,
            version,
        );

        if !package_files.is_empty() {
            let res = self
                .manager
                .lock()
                .await
                .add_packages_to_repo(package_files.clone())
                .await;
            if let Err(e) = &res {
                error!("Add to repository failed {}", e.to_string());
                package.set_status(PackageStatus::FAILED);
                package.last_error = Some("Failed to add package to repository".to_string());
            } else {
                info!("Added packages to repository {:?}", package_files);
            }
        }

        Ok(())
    }

    fn update_package_state_from_build_data(
        &mut self,
        package: &mut Package,
        package_files: Vec<String>,
        error: Option<String>,
        version: Option<String>,
    ) {
        package.set_last_built(Some(Utc::now()));
        if let Some(error) = error {
            package.set_status(PackageStatus::FAILED);
            package.last_error = Some(error.clone());
            error!("Error while building {}: '{}'", package.get_name(), error);
        } else {
            package.set_status(PackageStatus::BUILT);
            package.last_error = None;
            package.last_built_version = version;
            info!("Built {}", package.get_name());
        }

        *package.get_files_mut() = package_files;
    }

    // pub async fn rebuild_repo(&mut self) -> Result<()> {
    //     let packages = self.package_store.get_packages()?;
    //
    //     let files = packages.iter().fold(Vec::new(), |mut acc, i| {
    //         acc.append(&mut i.get_files().clone());
    //         acc
    //     });
    //
    //     if !files.is_empty() {
    //         return self.manager.lock().await.add_packages_to_repo(files).await;
    //     }
    //     Ok(())
    // }
}
