pub mod state;

use std::collections::HashMap;
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use log::{debug, error, info};
use tokio::sync::RwLock;
use tokio::time::sleep;
use common::models::WorkerStatus;
use common::models::PackageStatus;
use crate::models::config::Config;
use crate::models::worker::Worker;
use crate::orchestrator::state::State;

pub struct Orchestrator {
    pub workers: HashMap<usize, Worker>,
    pub state: State,

    is_running: Arc<AtomicBool>,
    rebuild_interval: Option<u64>,
    api_key: String,
}

impl Orchestrator {
    pub fn from_config(config: &Config) -> Orchestrator {
        Orchestrator {
            workers: HashMap::new(),
            state: State::new(config),

            is_running: Arc::new(AtomicBool::from(false)),
            rebuild_interval: config.rebuild_time,
            api_key: config.api_key.clone(),
        }
    }

    fn dispatch_packages(&mut self) {
        if let Some (rebuild_interval) = self.rebuild_interval {
            self.state.mark_package_for_rebuild(rebuild_interval);
        }

        while let Some(worker_id) = self.get_next_free_worker_id() {
            let package = self.state.get_next_pending_package();
            if let Some(package) = package {
                info!("Dispatch {} to worker {}", package.get_package_name(), worker_id);
                let worker = self.workers.get_mut(&worker_id).unwrap();
                let res = worker.dispatch_package(package);
                if let Err(e) = res {
                    error!("Failed to dispatch {} to worker {} with error : {}", package.get_package_name(), worker_id, e.to_string());
                }
            } else {
                return;
            }
        }
    }

    pub fn rebuild_all_packages(&mut self)
    {
        self.state.set_all_packages_pending();
    }

    pub fn try_authenticate_worker(&mut self, worker_id: &usize, submitted_key: String) -> bool {
        if let Some(worker) = self.workers.get_mut(worker_id) {
            if submitted_key == self.api_key {
                worker.authenticate();
                return true;
            }
        }
        return false;
    }

    fn get_next_free_worker_id(&self) -> Option<usize> {
        for (id, worker) in self.workers.iter() {
            if worker.get_status() == WorkerStatus::STANDBY && worker.is_authenticated() {
                return Some(id.clone());
            }
        }
        return None;
    }

    pub fn set_worker_status(&mut self, worker_id: usize, status: WorkerStatus, current_job: Option<String>)
    {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            worker.set_status(status);
            worker.set_current_job(current_job);
        }
    }

    pub fn remove_worker(&mut self, worker_id: usize)
    {
        debug!("Removing worker {}", worker_id);

        let worker = self.workers.remove(&worker_id);
        if let Some(worker) = worker {
            if let Some(current_job) = worker.get_current_job() {
                info!("Reverting {} back to PENDING because worker is getting removed", current_job);
                self.state.set_package_status(current_job, PackageStatus::PENDING);
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