mod manager;

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use chrono::{Duration, Utc};
use log::{debug, error, info, warn};
use std::fs::{OpenOptions};
use std::ops::Deref;
use tokio::fs::read_to_string;
use tokio::sync::{Mutex, RwLock};
use common::models::PackageStatus;
use crate::http::models::PackageBuildData;
use crate::http::multipart::MultipartField::File as MultipartFile;
use crate::models::config::Config;
use crate::models::package_state::PackageState;
use crate::models::server_package::ServerPackage;
use crate::repository::manager::RepositoryManager;

pub struct Repository {
    path: PathBuf,
    rebuild_interval: Option<u64>,

    packages: HashMap<String, ServerPackage>,
    manager: Arc<Mutex<RepositoryManager>>,
}

impl Repository
{
    pub async fn from_config(config: Arc<RwLock<Config>>) -> Result<Self, Box<dyn Error>>
    {
        let mut packages: HashMap<String, ServerPackage> = HashMap::new();

        for package in config.read().await.packages.iter() {
            if packages.contains_key(&package.name) {
                warn!("Found duplicate package entry for {}, skipping ...", package.name);
                continue;
            }

            let server_package = ServerPackage::from_package_definition(package.clone());
            packages.insert(server_package.get_package_name().clone(), server_package);
            debug!("Loaded package {}", package.name);
        }

        let manager = RepositoryManager::from_config(config.read().await.deref()).await?;

        Ok(Repository {
            path: PathBuf::from(config.read().await.get_serve_path()),
            rebuild_interval: config.read().await.rebuild_time,

            packages,
            manager: Arc::new(Mutex::new(manager))
        })
    }

    pub async fn try_restore_packages_states(&mut self) -> Result<(), Box<dyn Error>> {
        let contents = read_to_string(self.path.join("packages_state.json")).await?;
        let package_states: HashMap<String, PackageState> = serde_json::from_str(&contents)?;

        for (package_name, state) in package_states {
            if let Some(package) = self.packages.get_mut(&package_name) {
                info!("Restoring state for {}", package_name);
                for file in state.files.iter() {
                    if !self.path.join(file).exists() {
                        error!("Failed to restore package {}: Missing file '{}'", package_name, file);
                        continue;
                    }
                }
                package.state = state;
            } else {
                warn!("Dropping state for {}: not found in config", package_name);
            }
        }

        Ok(())
    }

    pub fn save_packages_states_to_disk(&self) {
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(self.path.join("packages_state.json"));

        match file {
            Ok(file) => {
                let states: HashMap<String, PackageState> = self.packages
                    .iter()
                    .map(|(name, package)| (name.clone(), package.state.clone())).collect();
                let res = serde_json::to_writer(file, &states);
                if let Err(e) = res {
                    error!("Failed to serialize state: {:?}", e);
                }
            }
            Err(err) => {
                error!("Failed to open file '{:?}': {:?}", self.path.join("packages_state.json"), err)
            }
        }
    }

    pub fn get_packages(&self) -> &HashMap<String, ServerPackage>
    {
        &self.packages
    }

    pub fn get_package_by_name(&self, package_name: &String) -> Option<&ServerPackage> {
        self.packages.get(package_name)
    }

    pub fn get_next_pending_package(&mut self) -> Option<&mut ServerPackage>
    {
        self.packages
            .iter_mut()
            .find(|(_, package)| package.state.status == PackageStatus::PENDING)
            .map(|it| it.1)
    }

    pub fn set_package_status(&mut self, package_name: &String, status: PackageStatus) {
        if let Some(package) = self.packages.get_mut(package_name) {
            package.set_status(status);
        }
        self.save_packages_states_to_disk();
    }

    pub fn queue_package_for_rebuild(&mut self, package_name: &String, force: bool) {
        if let Some(package) = self.packages.get_mut(package_name) {
            package.set_status(PackageStatus::PENDING);
            if force {
                package.clear_last_built_version();
            }
            info!(target: "Repository", "Queueing package {}; force: {}", package_name, force);
            self.save_packages_states_to_disk();
        }
    }

    pub fn queue_all_packages_for_rebuild(&mut self, force: bool) {
        for (_, package) in self.packages.iter_mut() {
            if package.state.status != PackageStatus::BUILDING {
                package.set_status(PackageStatus::PENDING);
                if force {
                    package.clear_last_built_version();
                }
            }
        }
        self.save_packages_states_to_disk();
    }

    pub fn check_rebuild_interval(&mut self)
    {
        if let Some(rebuild_interval) = self.rebuild_interval {
            for (_, package) in self.packages.iter_mut()
                .filter(|(_, it)| it.state.status == PackageStatus::FAILED || it.state.status == PackageStatus::BUILT)
            {
                if let Some(last_built) = package.state.last_built {
                    if Utc::now().signed_duration_since(last_built) > Duration::try_seconds(rebuild_interval as i64).unwrap() {
                        info!("Scheduled rebuild of package {}", package.get_package_name());
                        package.set_status(PackageStatus::PENDING);
                    }
                }
            }
            self.save_packages_states_to_disk();
        }
    }

    pub async fn handle_package_build_output(&mut self, package_name: &String, package_build_data: PackageBuildData<'_>) -> bool {
        if let Some(files) = package_build_data.log_files {
            for file in files.iter() {
                if let MultipartFile(filename, content) = file {
                    if let Err(e) = tokio::fs::write(format!("logs/{}", filename), content).await {
                        error!("Failed to write log file {}: {}", filename, e);
                    }
                }
            }
        }

        let mut package_files = Vec::new();
        if let Some(files) = package_build_data.files {
            for file in files.iter() {
                if let MultipartFile(filename, content) = file {
                    let path = self.path.join(filename);
                    debug!("Copying {filename} to {:?}...", path.as_path());
                    tokio::fs::write(path, content).await.unwrap();
                    package_files.push(filename.clone());
                }
            }
        }

        let is_updated = self.update_package_state_from_build_data(package_name, package_build_data, &package_files);

        if !package_files.is_empty() {
            let res = self.manager.lock().await.add_packages_to_repo(package_files.clone()).await;
            if let Err(e) = &res {
                error!("Add to repository failed {}", e.to_string());
                self.set_package_status(package_name, PackageStatus::FAILED);
            } else {
                info!("Added packages to repository {:?}", package_files);
            }
        }

        is_updated
    }

    fn update_package_state_from_build_data(
        &mut self,
        package_name: &String,
        build_data: PackageBuildData,
        package_files: &Vec<String>
    ) -> bool {
        let mut is_updated = false;

        if let Some(package) = self.packages.get_mut(package_name) {
            if let Some(error) = build_data.errors.first() {
                package.state.status = PackageStatus::FAILED;
                package.state.last_error = Some(error.clone());
                error!("Error while building {}: {}", package_name, error);
            } else {
                package.state.status = PackageStatus::BUILT;
                package.state.last_error = None;
                package.state.last_built = Some(Utc::now());
                package.state.last_built_version = build_data.version.clone();
                info!("Built {}", package_name);
            }

            if package.state.last_built_version == build_data.version || package.state.status == PackageStatus::FAILED {
                is_updated = true;
            }

            package.state.files = package_files.clone();
        }
        self.save_packages_states_to_disk();

        is_updated
    }

    pub async fn rebuild_repo(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut package_files = Vec::new();
        for (_, package) in self.packages.iter() {
            for file in package.state.files.iter() {
                package_files.push(file.clone());
            }
        }

        if !package_files.is_empty() {
            return self.manager.lock().await.add_packages_to_repo(package_files).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Sub;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use chrono::{TimeDelta, Utc};
    use serial_test::serial;
    use tokio::fs::{create_dir, remove_dir_all};
    use tokio::sync::Mutex;
    use common::models::{PackageDefinition, PackageStatus};
    use crate::http::models::PackageBuildData;
    use crate::http::multipart::MultipartField;
    use crate::models::server_package::ServerPackage;
    use crate::repository::manager::RepositoryManager;
    use crate::repository::Repository;

    async fn get_instance(reset: bool) -> Repository {
        if reset {
            let _ = remove_dir_all("/tmp/aur-build-server-test").await;
        }

        Repository {
            path: PathBuf::from("/tmp/aur-build-server-test/repo"),
            rebuild_interval: None,
            packages: Default::default(),
            manager:
            Arc::new(
                Mutex::new(
                    RepositoryManager::new("test".to_string(), None, "/tmp/aur-build-server-test/repo".to_string())
                .await.unwrap()
                )
            ),
        }
    }

    async fn get_instance_with_test_package(reset: bool) -> Repository {
        let mut instance = get_instance(reset).await;

        let package_definition = PackageDefinition {
            name: "test-package".to_string(),
            run_before: None,
            patches: None,
        };
        let package = ServerPackage::from_package_definition(package_definition.clone());
        instance.packages.insert("test-package".to_string(), package);
        instance
    }

    #[tokio::test]
    #[serial]
    async fn can_save_restore_state_test() {
        let mut repo = get_instance(true).await;

        let package_definition = PackageDefinition {
            name: "test-package".to_string(),
            run_before: None,
            patches: None,
        };
        let mut package = ServerPackage::from_package_definition(package_definition.clone());

        package.state.status = PackageStatus::BUILT;
        package.state.files = vec!["file1".to_string()];
        package.state.last_built = Some(Utc::now());
        package.state.last_built_version = Some("1.2.3".to_string());
        package.state.last_error = Some("error test".to_string());

        repo.packages.insert("test-package".to_string(), package.clone());
        repo.save_packages_states_to_disk();

        let mut repo = get_instance(false).await;
        repo.try_restore_packages_states().await.unwrap();

        // Should be empty if no previous entry
        assert_eq!(true, repo.get_packages().is_empty());

        repo.packages.insert("test-package".to_string(), ServerPackage::from_package_definition(package_definition));
        repo.try_restore_packages_states().await.unwrap();

        let restored_package = repo.get_packages().get(&"test-package".to_string()).unwrap();
        assert_eq!(package.state.status, restored_package.state.status);
        assert_eq!(package.state.files, restored_package.state.files);
        assert_eq!(package.state.last_built, restored_package.state.last_built);
        assert_eq!(package.state.last_built_version, restored_package.state.last_built_version);
        assert_eq!(package.state.last_error, restored_package.state.last_error);
    }

    #[tokio::test]
    #[serial]
    async fn retrieve_packages_test() {
        let instance = get_instance_with_test_package(true).await;

        assert_eq!(1, instance.get_packages().len());
        assert_eq!(
            "test-package",
            instance.get_package_by_name(&"test-package".to_string()).unwrap().get_package_name().as_str()
        );
    }

    #[tokio::test]
    #[serial]
    async fn get_next_pending_package_test() {
        let mut instance = get_instance_with_test_package(true).await;

        instance.set_package_status(&"test-package".to_string(), PackageStatus::BUILT);
        assert_eq!(true, instance.get_next_pending_package().is_none());

        instance.set_package_status(&"test-package".to_string(), PackageStatus::PENDING);
        assert_eq!("test-package", instance.get_next_pending_package().unwrap().get_package_name().as_str());
    }

    #[tokio::test]
    #[serial]
    async fn set_package_status_test() {
        let mut instance = get_instance_with_test_package(true).await;

        assert_eq!(false, instance.path.join("packages_state.json").exists());
        assert_eq!(PackageStatus::PENDING, instance.get_package_by_name(&"test-package".to_string()).unwrap().state.status);
        instance.set_package_status(&"test-package".to_string(), PackageStatus::BUILT);

        assert_eq!(true, instance.path.join("packages_state.json").exists());
        assert_eq!(PackageStatus::BUILT, instance.get_package_by_name(&"test-package".to_string()).unwrap().state.status);
    }

    #[tokio::test]
    #[serial]
    async fn queue_package_for_rebuild_test() {
        let mut instance = get_instance_with_test_package(true).await;
        instance.packages.get_mut(&"test-package".to_string()).unwrap().state.status = PackageStatus::BUILT;
        instance.packages.get_mut(&"test-package".to_string()).unwrap().state.last_built_version = Some("1.2.3".to_string());

        assert!(!instance.path.join("packages_state.json").exists());

        instance.queue_package_for_rebuild(&"no-package".to_string(), false); // Unknown package
        assert!(!instance.path.join("packages_state.json").exists());

        instance.queue_package_for_rebuild(&"test-package".to_string(), false);
        assert!(instance.path.join("packages_state.json").exists());
        assert!(instance.get_package_by_name(&"test-package".to_string()).unwrap().state.last_built_version.is_some());
        assert_eq!(PackageStatus::PENDING, instance.get_package_by_name(&"test-package".to_string()).unwrap().state.status);

        instance.queue_package_for_rebuild(&"test-package".to_string(), true);
        assert!(instance.get_package_by_name(&"test-package".to_string()).unwrap().state.last_built_version.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn check_rebuild_interval_test() {
        let mut instance = get_instance_with_test_package(true).await;
        instance.rebuild_interval = Some(100);
        instance.packages.get_mut(&"test-package".to_string()).unwrap().state.status = PackageStatus::BUILT;
        instance.packages.get_mut(&"test-package".to_string()).unwrap().state.last_built = Some(Utc::now());

        instance.check_rebuild_interval();
        assert_eq!(PackageStatus::BUILT, instance.get_package_by_name(&"test-package".to_string()).unwrap().state.status);

        instance.packages.get_mut(&"test-package".to_string()).unwrap().state.last_built = Some(Utc::now().sub(TimeDelta::try_seconds(200).unwrap()));
        instance.check_rebuild_interval();
        assert_eq!(PackageStatus::PENDING, instance.get_package_by_name(&"test-package".to_string()).unwrap().state.status);
    }

    #[tokio::test]
    #[serial]
    async fn handle_package_build_output_success_test() {
        let mut instance = get_instance_with_test_package(true).await;
        create_dir("logs").await;

        let files = vec![
            MultipartField::File(
                "aur-build-cli-0.10.0-1-any.pkg.tar.zst".into(),
                tokio::fs::read("tests/aur-build-cli-0.10.0-1-any.pkg.tar.zst").await.unwrap()
            )
        ];
        let log_files = vec![
            MultipartField::File(
                "log-file.log".into(),
                "test log file".as_bytes().to_vec()
            )
        ];

        let build_data = PackageBuildData {
            files: Some(&files),
            log_files: Some(&log_files),
            errors: vec![],
            version: Some("11.2.3".to_string()),
        };
        let is_updated = instance.handle_package_build_output(&"test-package".to_string(), build_data).await;

        assert!(instance.path.join("aur-build-cli-0.10.0-1-any.pkg.tar.zst").exists());
        let package = instance.get_package_by_name(&"test-package".to_string()).unwrap();
        assert_eq!(PackageStatus::BUILT, package.state.status);
        assert!(package.state.last_built.is_some());
        assert_eq!("11.2.3", package.state.last_built_version.as_ref().unwrap().as_str());
        assert_eq!(1, package.state.files.len());
        assert_eq!("aur-build-cli-0.10.0-1-any.pkg.tar.zst", package.state.files[0].as_str());
        assert!(Path::new("logs/log-file.log").exists());

        let database_path = PathBuf::from("/tmp/aur-build-server-test/repo/test.db");
        assert!(database_path.exists());

        remove_dir_all("logs").await.unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn handle_package_build_output_fail_test() {
        let mut instance = get_instance_with_test_package(true).await;
        let _ = create_dir("logs").await;

        let files = vec![];
        let log_files = vec![
            MultipartField::File(
                "log-file.log".into(),
                "test log file".as_bytes().to_vec()
            )
        ];

        let build_data = PackageBuildData {
            files: Some(&files),
            log_files: Some(&log_files),
            errors: vec!["Error test".into()],
            version: Some("11.2.3".to_string()),
        };
        let is_updated = instance.handle_package_build_output(&"test-package".to_string(), build_data).await;

        let package = instance.get_package_by_name(&"test-package".to_string()).unwrap();
        assert_eq!(PackageStatus::FAILED, package.state.status);
        assert!(package.state.last_built.is_none());
        assert!(package.state.last_built_version.is_none());
        assert_eq!(0, package.state.files.len());
        assert!(Path::new("logs/log-file.log").exists());

        remove_dir_all("logs").await.unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn rebuild_repo_test() {
        let mut instance = get_instance_with_test_package(true).await;

        tokio::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("aur-build-cli-0.10.0-1-any.pkg.tar.zst"),
            "/tmp/aur-build-server-test/repo/aur-build-cli-0.10.0-1-any.pkg.tar.zst"
        ).await.unwrap();

        instance.packages.get_mut(&"test-package".to_string())
            .unwrap()
            .state.files.push("aur-build-cli-0.10.0-1-any.pkg.tar.zst".into());

        let database_path = PathBuf::from("/tmp/aur-build-server-test/repo/test.db");
        assert!(!database_path.exists());

        let rebuild_repo = instance.rebuild_repo().await;

        assert!(rebuild_repo.is_ok());
        assert!(database_path.exists());
    }

}