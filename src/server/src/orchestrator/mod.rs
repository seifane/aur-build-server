use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use log::{debug, error, info};
use tokio::sync::{RwLock};
use tokio::time::sleep;
use common::models::PackageStatus;
use crate::models::config::Config;
use crate::repository::Repository;
use crate::webhooks::WebhookManager;
use crate::worker::manager::{WorkerDispatchResult, WorkerManager};

pub struct Orchestrator {
    pub worker_manager: WorkerManager,
    pub webhook_manager: WebhookManager,
    pub repository: Repository,

    pub config: Arc<RwLock<Config>>,

    is_running: Arc<AtomicBool>,
}

impl Orchestrator {
    pub async fn new(config: Arc<RwLock<Config>>) -> Orchestrator {
        Orchestrator {
            worker_manager: WorkerManager::new(config.clone()),
            webhook_manager: WebhookManager::from_config(config.clone()),
            repository: Repository::from_config(config.clone()).await.unwrap(),

            is_running: Arc::new(AtomicBool::from(false)),

            config
        }
    }

    pub async fn restore_state(&mut self) {
        match self.repository.try_restore_packages_states().await {
            Ok(_) => {
                info!("Restored state for packages");
                match self.repository.rebuild_repo().await {
                    Ok(_) => info!("Rebuilt repository"),
                    Err(e) => error!("Failed to rebuild repository: {:?}", e)
                }
            }
            Err(e) => {
                error!("Failed to restore state for packages: {:?}", e);
            }
        };
    }

    pub fn remove_worker(&mut self, worker_id: usize)
    {
        debug!("Removing worker {}", worker_id);
        let worker = self.worker_manager.remove(worker_id);
        if let Some(worker) = worker {
            if let Some(current_job) = worker.get_current_job() {
                info!("Reverting {} back to PENDING because worker is getting removed", current_job);
                self.repository.set_package_status(current_job, PackageStatus::PENDING);
            }
        }
    }

    fn dispatch_packages(&mut self) {
        self.repository.check_rebuild_interval();

        while let Some(package) = self.repository.get_next_pending_package() {
            match self.worker_manager.dispatch(package) {
                WorkerDispatchResult::NoneAvailable => return,
                WorkerDispatchResult::Ok => {
                    debug!("Dispatched package {} to worker", package.get_package_name());
                }
                WorkerDispatchResult::Err(e) => {
                    error!("Error while dispatching {} to worker : {}", package.get_package_name(), e)
                }
            }
        }
    }

    pub async fn dispatch_loop(orchestrator: Arc<RwLock<Orchestrator>>) {
        let is_running = orchestrator.read().await.is_running.clone();

        is_running.store(true, Ordering::SeqCst);

        while is_running.load(Ordering::SeqCst) {
            orchestrator.write().await.dispatch_packages();
            sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}