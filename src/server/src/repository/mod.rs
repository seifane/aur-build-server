mod manager;

use crate::models::config::Config;
use crate::persistence::package_store::{Package};
use crate::repository::manager::RepositoryManager;
use anyhow::{Result};
use chrono::Utc;
use common::models::PackageStatus;
use log::{error, info, warn};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use actix_multipart::form::tempfile::TempFile;
use tokio::sync::{Mutex, RwLock};

pub struct Repository {
    path: PathBuf,
    build_logs_path: PathBuf,

    manager: Arc<Mutex<RepositoryManager>>,
}

impl Repository {
    pub async fn from_config(config: Arc<RwLock<Config>>) -> Result<Self> {
        // for package in config.read().await.packages.iter() {
        //     if packages.contains_key(&package.name) {
        //         warn!(
        //             "Found duplicate package entry for {}, skipping ...",
        //             package.name
        //         );
        //         continue;
        //     }
        //
        //     let server_package = ServerPackage::from_package_definition(package.clone());
        //     packages.insert(server_package.get_package_name().clone(), server_package);
        //     debug!("Loaded package {}", package.name);
        // }

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
                match log_file.file.persist(&self.build_logs_path.join(&filename)) {
                    Ok(_) => info!("Successfully persisted log file '{}'", filename),
                    Err(e) => error!("Unable to persist log file '{}': '{}'", filename, e),
                };
            }
        }
        let mut package_files = Vec::new();

        for file in files {
            if let Some(filename) = file.file_name {
                match file.file.persist(&self.path.join(&filename)) {
                    Ok(_) => info!("Successfully package file '{}'", filename),
                    Err(e) => error!("Unable to persist package file '{}': '{}'", filename, e),
                }
                package_files.push(filename)
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

// #[cfg(test)]
// mod tests {
//     use crate::http::models::PackageBuildData;
//     use crate::http::multipart::MultipartField;
//     use crate::models::server_package::ServerPackage;
//     use crate::repository::manager::RepositoryManager;
//     use crate::repository::Repository;
//     use chrono::{TimeDelta, Utc};
//     use common::models::{PackageDefinition, PackageStatus};
//     use serial_test::serial;
//     use std::ops::Sub;
//     use std::path::{Path, PathBuf};
//     use std::sync::Arc;
//     use tokio::fs::{create_dir, remove_dir_all};
//     use tokio::sync::Mutex;
//
//     async fn get_instance(reset: bool) -> Repository {
//         if reset {
//             let _ = remove_dir_all("/tmp/aur-build-server-test").await;
//         }
//
//         Repository {
//             path: PathBuf::from("/tmp/aur-build-server-test/repo"),
//             build_logs_path: PathBuf::from("/tmp/aur-build-server-test/logs"),
//             rebuild_interval: None,
//             packages: Default::default(),
//             manager: Arc::new(Mutex::new(
//                 RepositoryManager::new(
//                     "test".to_string(),
//                     None,
//                     PathBuf::from("/tmp/aur-build-server-test/repo"),
//                 )
//                 .await
//                 .unwrap(),
//             )),
//         }
//     }
//
//     async fn get_instance_with_test_package(reset: bool) -> Repository {
//         let mut instance = get_instance(reset).await;
//
//         let package_definition = PackageDefinition {
//             name: "test-package".to_string(),
//             run_before: None,
//             patches: None,
//         };
//         let package = ServerPackage::from_package_definition(package_definition.clone());
//         instance
//             .packages
//             .insert("test-package".to_string(), package);
//         instance
//     }
//
//     #[tokio::test]
//     #[serial]
//     async fn can_save_restore_state_test() {
//         let mut repo = get_instance(true).await;
//
//         let package_definition = PackageDefinition {
//             name: "test-package".to_string(),
//             run_before: None,
//             patches: None,
//         };
//         let mut package = ServerPackage::from_package_definition(package_definition.clone());
//
//         package.state.status = PackageStatus::BUILT;
//         package.state.files = vec!["file1".to_string()];
//         package.state.last_built = Some(Utc::now());
//         package.state.last_built_version = Some("1.2.3".to_string());
//         package.state.last_error = Some("error test".to_string());
//
//         repo.packages
//             .insert("test-package".to_string(), package.clone());
//
//         let mut repo = get_instance(false).await;
//
//         // Should be empty if no previous entry
//         assert_eq!(
//             true,
//             repo.get_package_store().get_packages().unwrap().is_empty()
//         );
//
//         repo.packages.insert(
//             "test-package".to_string(),
//             ServerPackage::from_package_definition(package_definition),
//         );
//
//         let restored_package = repo
//             .get_package_store()
//             .get_packages()
//             .get(&"test-package".to_string())
//             .unwrap();
//         assert_eq!(package.state.status, restored_package.state.status);
//         assert_eq!(package.state.files, restored_package.state.files);
//         assert_eq!(package.state.last_built, restored_package.state.last_built);
//         assert_eq!(
//             package.state.last_built_version,
//             restored_package.state.last_built_version
//         );
//         assert_eq!(package.state.last_error, restored_package.state.last_error);
//     }
//
//     #[tokio::test]
//     #[serial]
//     async fn retrieve_packages_test() {
//         let instance = get_instance_with_test_package(true).await;
//
//         assert_eq!(1, instance.get_packages().len());
//         assert_eq!(
//             "test-package",
//             instance
//                 .get_package_by_name(&"test-package".to_string())
//                 .unwrap()
//                 .get_package_name()
//                 .as_str()
//         );
//     }
//
//     #[tokio::test]
//     #[serial]
//     async fn get_next_pending_package_test() {
//         let mut instance = get_instance_with_test_package(true).await;
//
//         instance.set_package_status(&"test-package".to_string(), PackageStatus::BUILT);
//         assert_eq!(true, instance.get_next_pending_package().is_none());
//
//         instance.set_package_status(&"test-package".to_string(), PackageStatus::PENDING);
//         assert_eq!(
//             "test-package",
//             instance
//                 .get_next_pending_package()
//                 .unwrap()
//                 .get_package_name()
//                 .as_str()
//         );
//     }
//
//     #[tokio::test]
//     #[serial]
//     async fn set_package_status_test() {
//         let mut instance = get_instance_with_test_package(true).await;
//
//         assert_eq!(false, instance.path.join("packages_state.json").exists());
//         assert_eq!(
//             PackageStatus::PENDING,
//             instance
//                 .get_package_by_name(&"test-package".to_string())
//                 .unwrap()
//                 .state
//                 .status
//         );
//         instance.set_package_status(&"test-package".to_string(), PackageStatus::BUILT);
//
//         assert_eq!(true, instance.path.join("packages_state.json").exists());
//         assert_eq!(
//             PackageStatus::BUILT,
//             instance
//                 .get_package_by_name(&"test-package".to_string())
//                 .unwrap()
//                 .state
//                 .status
//         );
//     }
//
//     #[tokio::test]
//     #[serial]
//     async fn queue_package_for_rebuild_test() {
//         let mut instance = get_instance_with_test_package(true).await;
//         instance
//             .packages
//             .get_mut(&"test-package".to_string())
//             .unwrap()
//             .state
//             .status = PackageStatus::BUILT;
//         instance
//             .packages
//             .get_mut(&"test-package".to_string())
//             .unwrap()
//             .state
//             .last_built_version = Some("1.2.3".to_string());
//
//         assert!(!instance.path.join("packages_state.json").exists());
//
//         instance.queue_package_for_rebuild(&"no-package".to_string(), false); // Unknown package
//         assert!(!instance.path.join("packages_state.json").exists());
//
//         instance.queue_package_for_rebuild(&"test-package".to_string(), false);
//         assert!(instance.path.join("packages_state.json").exists());
//         assert!(instance
//             .get_package_by_name(&"test-package".to_string())
//             .unwrap()
//             .state
//             .last_built_version
//             .is_some());
//         assert_eq!(
//             PackageStatus::PENDING,
//             instance
//                 .get_package_by_name(&"test-package".to_string())
//                 .unwrap()
//                 .state
//                 .status
//         );
//
//         instance.queue_package_for_rebuild(&"test-package".to_string(), true);
//         assert!(instance
//             .get_package_by_name(&"test-package".to_string())
//             .unwrap()
//             .state
//             .last_built_version
//             .is_none());
//     }
//
//     #[tokio::test]
//     #[serial]
//     async fn check_rebuild_interval_test() {
//         let mut instance = get_instance_with_test_package(true).await;
//         instance.rebuild_interval = Some(100);
//         instance
//             .packages
//             .get_mut(&"test-package".to_string())
//             .unwrap()
//             .state
//             .status = PackageStatus::BUILT;
//         instance
//             .packages
//             .get_mut(&"test-package".to_string())
//             .unwrap()
//             .state
//             .last_built = Some(Utc::now());
//
//         instance.check_rebuild_interval();
//         assert_eq!(
//             PackageStatus::BUILT,
//             instance
//                 .get_package_by_name(&"test-package".to_string())
//                 .unwrap()
//                 .state
//                 .status
//         );
//
//         instance
//             .packages
//             .get_mut(&"test-package".to_string())
//             .unwrap()
//             .state
//             .last_built = Some(Utc::now().sub(TimeDelta::try_seconds(200).unwrap()));
//         instance.check_rebuild_interval();
//         assert_eq!(
//             PackageStatus::PENDING,
//             instance
//                 .get_package_by_name(&"test-package".to_string())
//                 .unwrap()
//                 .state
//                 .status
//         );
//     }
//
//     #[tokio::test]
//     #[serial]
//     async fn handle_package_build_output_success_test() {
//         let mut instance = get_instance_with_test_package(true).await;
//         let _ = create_dir("logs").await;
//
//         let files = vec![MultipartField::File(
//             "aur-build-cli-0.10.0-1-any.pkg.tar.zst".into(),
//             tokio::fs::read("tests/aur-build-cli-0.10.0-1-any.pkg.tar.zst")
//                 .await
//                 .unwrap(),
//         )];
//         let log_files = vec![MultipartField::File(
//             "log-file.log".into(),
//             "test log file".as_bytes().to_vec(),
//         )];
//
//         let build_data = PackageBuildData {
//             files: Some(&files),
//             log_files: Some(&log_files),
//             errors: vec![],
//             version: Some("11.2.3".to_string()),
//         };
//         let is_updated = instance
//             .handle_package_build_output(&"test-package".to_string(), build_data)
//             .await;
//
//         assert!(is_updated);
//         assert!(instance
//             .path
//             .join("aur-build-cli-0.10.0-1-any.pkg.tar.zst")
//             .exists());
//         let package = instance
//             .get_package_by_name(&"test-package".to_string())
//             .unwrap();
//         assert_eq!(PackageStatus::BUILT, package.state.status);
//         assert!(package.state.last_built.is_some());
//         assert_eq!(
//             "11.2.3",
//             package.state.last_built_version.as_ref().unwrap().as_str()
//         );
//         assert_eq!(1, package.state.files.len());
//         assert_eq!(
//             "aur-build-cli-0.10.0-1-any.pkg.tar.zst",
//             package.state.files[0].as_str()
//         );
//         assert!(Path::new("/tmp/aur-build-server-test/logs/log-file.log").exists());
//
//         let database_path = PathBuf::from("/tmp/aur-build-server-test/repo/test.db");
//         assert!(database_path.exists());
//
//         remove_dir_all("logs").await.unwrap();
//     }
//
//     #[tokio::test]
//     #[serial]
//     async fn handle_package_build_output_fail_test() {
//         let mut instance = get_instance_with_test_package(true).await;
//         let _ = create_dir("logs").await;
//
//         let files = vec![];
//         let log_files = vec![MultipartField::File(
//             "log-file.log".into(),
//             "test log file".as_bytes().to_vec(),
//         )];
//
//         let build_data = PackageBuildData {
//             files: Some(&files),
//             log_files: Some(&log_files),
//             errors: vec!["Error test".into()],
//             version: Some("11.2.3".to_string()),
//         };
//         let is_updated = instance
//             .handle_package_build_output(&"test-package".to_string(), build_data)
//             .await;
//
//         assert!(is_updated);
//         let package = instance
//             .get_package_by_name(&"test-package".to_string())
//             .unwrap();
//         assert_eq!(PackageStatus::FAILED, package.state.status);
//         assert!(package.state.last_built.is_some());
//         assert!(package.state.last_built_version.is_none());
//         assert_eq!(0, package.state.files.len());
//         assert!(Path::new("/tmp/aur-build-server-test/logs/log-file.log").exists());
//
//         remove_dir_all("logs").await.unwrap();
//     }
//
//     #[tokio::test]
//     #[serial]
//     async fn rebuild_repo_test() {
//         let mut instance = get_instance_with_test_package(true).await;
//
//         tokio::fs::copy(
//             PathBuf::from(env!("CARGO_MANIFEST_DIR"))
//                 .join("tests")
//                 .join("aur-build-cli-0.10.0-1-any.pkg.tar.zst"),
//             "/tmp/aur-build-server-test/repo/aur-build-cli-0.10.0-1-any.pkg.tar.zst",
//         )
//         .await
//         .unwrap();
//
//         instance
//             .packages
//             .get_mut(&"test-package".to_string())
//             .unwrap()
//             .state
//             .files
//             .push("aur-build-cli-0.10.0-1-any.pkg.tar.zst".into());
//
//         let database_path = PathBuf::from("/tmp/aur-build-server-test/repo/test.db");
//         assert!(!database_path.exists());
//
//         let rebuild_repo = instance.rebuild_repo().await;
//
//         assert!(rebuild_repo.is_ok());
//         assert!(database_path.exists());
//     }
// }
