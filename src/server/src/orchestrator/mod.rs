pub mod state;

use std::collections::HashMap;
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use log::{debug, error, info, warn};
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;
use common::models::WorkerStatus;
use common::models::PackageStatus;
use crate::http::util::MultipartField::File;
use crate::models::config::Config;
use crate::models::worker::Worker;
use crate::orchestrator::state::{PackageBuildData, State};
use crate::utils::repo::Repo;
use crate::webhooks::WebhookManager;

pub struct Orchestrator {
    pub workers: HashMap<usize, Worker>,
    pub state: State,
    pub webhook_manager: WebhookManager,

    repo: Arc<Mutex<Repo>>,

    is_running: Arc<AtomicBool>,
    rebuild_interval: Option<u64>,
    api_key: String,
}

impl Orchestrator {
    pub fn new(repo: Arc<Mutex<Repo>>, config: &Config) -> Orchestrator {
        Orchestrator {
            workers: HashMap::new(),
            state: State::new(config),
            webhook_manager: WebhookManager::from_config(config),

            repo,

            is_running: Arc::new(AtomicBool::from(false)),
            rebuild_interval: config.rebuild_time,
            api_key: config.api_key.clone(),
        }
    }

    pub async fn restore_state(&mut self) {
        match self.state.restore() {
            Ok(_) => {
                info!("Restored state for packages");
                let mut package_files = Vec::new();
                for (_, package) in self.state.get_packages().iter() {
                    package_files.append(&mut package.state.files.clone())
                }
                debug!("Packages restored from state {:?}", package_files);

                if let Err(err) = self.repo.lock().await.set_repo_packages(package_files).await {
                    warn!("Error while setting repo packages from state: {:?}", err);
                }
            },
            Err(e) => {
                error!("Failed to restore state for packages: {:?}", e);
            }
        };
    }

    pub fn rebuild_all_packages(&mut self)
    {
        self.state.set_all_packages_pending();
    }

    pub async fn handle_package_build_response(&mut self, package_name: &String, package_build_data: PackageBuildData<'_>)
    {
        if let Some(files) = package_build_data.log_files {
            for file in files.iter() {
                if let File(filename, content) = file {
                    tokio::fs::write(format!("logs/{}", filename), content).await.unwrap();
                }
            }
        }

        let mut package_files = Vec::new();
        if let Some(files) = package_build_data.files {
            for file in files.iter() {
                if let File(filename, content) = file {
                    debug!("Copying {} ...", filename);
                    let path = self.repo.lock().await.path.join(filename).to_str().unwrap().to_string();
                    tokio::fs::write(path, content).await.unwrap();
                    package_files.push(filename.clone());
                }
            }
        }
        info!("Received packages {:?}", package_files);

        if self.state.update_package_state_from_build_data(package_name, package_build_data, package_files.clone()) {
            self.webhook_manager.trigger_webhook_package_updated(self.state.get_package_by_name(package_name).unwrap().as_http_response()).await;
        }

        if !package_files.is_empty() {
            let res = self.repo.lock().await.add_packages_to_repo(package_files.clone()).await;
            if let Err(e) = &res {
                error!("Add to repo failed {}", e.to_string());
                self.state.set_package_status(package_name, PackageStatus::FAILED);
            } else {
                info!("Added packages to repo {:?}", package_files);
            }
        }
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

    pub async fn dispatch_loop(orchestrator: Arc<RwLock<Orchestrator>>) {
        let is_running = orchestrator.read().await.is_running.clone();

        is_running.store(true, Ordering::SeqCst);

        while is_running.load(Ordering::SeqCst) {
            orchestrator.write().await.dispatch_packages();
            sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}