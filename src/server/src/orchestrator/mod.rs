use crate::models::config::Config;
use crate::persistence::package_store::{PackageInsert, PackageStore};
use crate::repository::Repository;
use crate::webhooks::WebhookManager;
use crate::worker::worker_manager::{WorkerDispatchResult, WorkerManager};
use anyhow::Result;
use common::models::PackageStatus;
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use actix_multipart::form::tempfile::TempFile;
use actix_ws::{AggregatedMessageStream, Session};
use tokio::sync::RwLock;
use tokio::time::sleep;
use crate::worker::worker::Worker;

pub struct Orchestrator {
    worker_manager: WorkerManager,
    pub webhook_manager: WebhookManager,
    repository: Repository,

    package_store: PackageStore,
    rebuild_interval: Option<u64>,
    is_running: Arc<AtomicBool>,

    pub config: Arc<RwLock<Config>>,
}

impl Orchestrator {
    pub async fn new(config: Arc<RwLock<Config>>) -> Result<Orchestrator> {
        let (database_path, rebuild_interval) = {
            let config = config.read().await;
            (config.database_path.clone(), config.rebuild_time.clone())
        };

        let mut package_store = PackageStore::from_disk(database_path)?;
        package_store.run_migrations().await?;

        let packages = package_store.get_packages().await?;
        for package in config.read().await.packages.iter() {
            if packages.iter().find(|i| i.get_name() == &package.name).is_none() {
                println!("Temp add packages to store");
                package_store.create_package(PackageInsert {
                    name: package.name.clone(),
                    run_before: package.run_before.clone(),
                }).await.unwrap();
            }
        }

        Ok(Orchestrator {
            worker_manager: WorkerManager::new(config.clone()),
            webhook_manager: WebhookManager::from_config(config.clone()),
            repository: Repository::from_config(config.clone()).await?,

            package_store,

            rebuild_interval,
            is_running: Arc::new(AtomicBool::from(false)),

            config,
        })
    }

    pub fn get_package_store(&mut self) -> &mut PackageStore {
        &mut self.package_store
    }

    pub fn get_worker_manager(&self) -> &WorkerManager {
        &self.worker_manager
    }

    pub async fn add_worker(&mut self, session: Session, stream: AggregatedMessageStream)
    {
        self.worker_manager.add(session, stream).await;
    }

    pub async fn remove_worker(&mut self, worker_id: usize) {
        if let Some(worker) = self.worker_manager.remove(worker_id) {
            self.handle_removed_worker(worker).await;
        }
    }

    pub async fn remove_finished_workers(&mut self) {
        let finished_workers = self.worker_manager.remove_finished_workers();
        for worker in finished_workers {
            self.handle_removed_worker(worker).await;
        }
    }

    async fn handle_removed_worker(&mut self, worker: Worker) {
        info!("Removing worker {}", worker.get_id());
        if let Some(current_job) = worker.get_current_job().await {
            info!(
                "Reverting {} back to PENDING because worker is getting removed",
                current_job.definition.name
            );
            if let Err(e) = self.get_package_store().update_package_status(
                current_job.definition.package_id,
                PackageStatus::PENDING,
            ).await {
                error!(
                    "Failed to reset package status {}: '{}'",
                    current_job.definition.package_id, e
                );
            };
        }
    }

    pub async fn handle_package_build_output(
        &mut self,
        package_name: String,
        version: Option<String>,
        error: Option<String>,
        log_files: Vec<TempFile>,
        files: Vec<TempFile>,
    ) -> Result<()> {
        if let Some(mut package) = self.package_store.get_package_by_name(&package_name).await? {
            let last_version = package.last_built_version.clone();
            self.repository.handle_package_build_output(&mut package, version, error, log_files, files).await?;
            self.package_store.update_package(&package).await?;
            if package.last_built_version != last_version {
                self.webhook_manager.trigger_webhook_package_updated(package.into()).await;
            }
        }
        Ok(())
    }

    async fn dispatch_packages(&mut self) -> Result<()> {
        if let Some(rebuild_interval) = self.rebuild_interval {
            self.package_store
                .set_packages_rebuild(rebuild_interval as i64).await?;
        }

        while let Some(mut package) = self.package_store.get_next_pending_package().await? {
            let patches = self.package_store.get_patches_for_package(package.get_id()).await?;
            match self.worker_manager.dispatch(package.get_package_job(patches)).await {
                WorkerDispatchResult::NoneAvailable => {
                    println!("No workers");
                    return Ok(())
                },
                WorkerDispatchResult::Ok => {
                    package.set_status(PackageStatus::BUILDING);
                    self.package_store.update_package(&package).await?;
                    info!("Dispatched package {} to worker", package.get_name());
                }
                WorkerDispatchResult::Err(e) => {
                    error!(
                        "Error while dispatching {} to worker : {}",
                        package.get_name(),
                        e
                    )
                }
            }
        }

        Ok(())
    }

    pub async fn dispatch_loop(orchestrator: Arc<RwLock<Orchestrator>>) {
        let is_running = orchestrator.read().await.is_running.clone();

        is_running.store(true, Ordering::SeqCst);

        while is_running.load(Ordering::SeqCst) {
            println!("running");
            if let Err(e) = orchestrator.write().await.dispatch_packages().await {
                error!("Error while dispatching packages : {}", e);
            }
            orchestrator.write().await.remove_finished_workers().await;
            sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::config::Config;
    use crate::orchestrator::Orchestrator;
    use crate::persistence::package_store::PackageInsert;
    use log::LevelFilter;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use actix_multipart::form::tempfile::TempFile;
    use serial_test::serial;
    use std::io::Write;
    use tokio::fs::create_dir_all;
    use tokio::sync::RwLock;
    use common::models::PackageStatus;

    async fn get_instance() -> Orchestrator {
        let config = Config {
            log_level: LevelFilter::Off,
            log_path: PathBuf::from("/tmp/aur-build-server-test/log.txt"),
            api_key: "api_key".to_string(),
            port: 3000,
            repo_name: "test".to_string(),
            sign_key: None,
            rebuild_time: None,
            serve_path: PathBuf::from("/tmp/aur-build-server-test/repo"),
            build_logs_path: PathBuf::from("/tmp/aur-build-server-test/logs"),
            database_path: ":memory:".into(),
            webhooks: vec![],
            packages: vec![],
        };
        let mut orchestrator = Orchestrator::new(Arc::new(RwLock::new(config))).await.unwrap();

        orchestrator.package_store.run_migrations().await.unwrap();
        orchestrator.package_store.create_package(PackageInsert {
            name: "test-package".to_string(),
            run_before: None,
        }).await.unwrap();

        let test_dir = Path::new("/tmp/aur-build-server-test");
        if test_dir.exists() {
            std::fs::remove_dir_all(test_dir).unwrap();
            create_dir_all(test_dir).await.unwrap();
            create_dir_all(&orchestrator.config.read().await.serve_path).await.unwrap();
            create_dir_all(&orchestrator.config.read().await.build_logs_path).await.unwrap();
        }

        orchestrator
    }

    #[tokio::test]
    #[serial]
    async fn handle_package_build_output_success_test() {
        let mut orchestrator = get_instance().await;

        let mut file = tempfile::Builder::new()
            .prefix("aur-build-cli-0.10.0-1-any")
            .suffix(".pkg.tar.zst")
            .tempfile().unwrap();
        file.write_all(&tokio::fs::read("tests/aur-build-cli-0.10.0-1-any.pkg.tar.zst").await.unwrap()).unwrap();
        let package_file = TempFile {
            file,
            content_type: None,
            file_name: Some("aur-build-cli-0.10.0-1-any.pkg.tar.zst".to_string()),
            size: 1
        };

        let mut file = tempfile::Builder::new()
            .tempfile().unwrap();
        writeln!(file, "log-content").unwrap();
        let log_file = TempFile {
            file,
            content_type: None,
            file_name: Some("test-package.log".to_string()),
            size: 1
        };

        orchestrator
            .handle_package_build_output(
                "test-package".to_string(),
                Some("11.2.3".to_string()),
                None,
                vec![log_file],
                vec![package_file])
            .await.unwrap();

        assert!(orchestrator
            .config.read().await.serve_path
            .join("aur-build-cli-0.10.0-1-any.pkg.tar.zst")
            .exists());

        let package = orchestrator
            .package_store
            .get_package_by_name(&"test-package".to_string()).await
            .unwrap().unwrap();
        assert_eq!(PackageStatus::BUILT, package.get_status());
        assert!(package.get_last_built().is_some());
        assert_eq!(
            "11.2.3",
            package.last_built_version.as_ref().unwrap().as_str()
        );
        assert_eq!(1, package.get_files().len());
        assert_eq!(
            "aur-build-cli-0.10.0-1-any.pkg.tar.zst",
            package.get_files()[0].as_str()
        );
        assert!(Path::new("/tmp/aur-build-server-test/logs/test-package.log").exists());

        let database_path = PathBuf::from("/tmp/aur-build-server-test/repo/test.db");
        assert!(database_path.exists());
    }

    #[tokio::test]
    #[serial]
    async fn handle_package_build_output_fail_test() {
        let mut orchestrator = get_instance().await;

        let mut file = tempfile::Builder::new()
            .tempfile().unwrap();
        writeln!(file, "log-content").unwrap();
        let log_file = TempFile {
            file,
            content_type: None,
            file_name: Some("test-package.log".to_string()),
            size: 1
        };

        orchestrator
            .handle_package_build_output(
                "test-package".to_string(),
                Some("11.2.3".to_string()),
                Some("Error test".to_string()),
                vec![log_file],
                vec![])
            .await.unwrap();

        let package = orchestrator.package_store
            .get_package_by_name(&"test-package".to_string()).await
            .unwrap().unwrap();
        assert_eq!(PackageStatus::FAILED, package.get_status());
        assert!(package.get_last_built().is_some());
        assert!(package.last_built_version.is_none());
        assert_eq!(0, package.get_files().len());
        assert!(Path::new("/tmp/aur-build-server-test/logs/test-package.log").exists());
    }
}