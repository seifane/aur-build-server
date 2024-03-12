use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use chrono::{DateTime, Duration};
use log::{debug, info};
use chrono::offset::Utc;
use tokio::sync::RwLock;
use tokio::time::sleep;
use common::models::WorkerStatus;
use common::models::PackageStatus;
use crate::models::config::Config;
use crate::models::server_package::{ServerPackage};
use crate::models::worker::Worker;

pub struct Orchestrator {
    pub workers: HashMap<usize, Worker>,
    pub packages: Vec<ServerPackage>,

    is_running: Arc<AtomicBool>,
    rebuild_interval: Option<u64>,
    api_key: String,
}

impl Orchestrator {
    pub fn from_config(config: &Config) -> Orchestrator {
        let mut packages = Vec::new();

        for package in config.packages.iter() {
            let package = ServerPackage::from_package_config(package);
            debug!("Loaded package {}", package.get_package_name());
            packages.push(package);
        }

        Orchestrator {
            workers: HashMap::new(),
            packages,

            is_running: Arc::new(AtomicBool::from(false)),
            rebuild_interval: config.rebuild_time,
            api_key: config.api_key.clone(),
        }
    }

    fn get_next_free_worker_id(&self) -> Option<usize> {
        for (id, worker) in self.workers.iter() {
            if worker.get_status() == WorkerStatus::STANDBY && worker.is_authenticated() {
                return Some(id.clone());
            }
        }
        return None;
    }

    fn dispatch_packages(&mut self) {
        for index in 0..self.packages.len() {
            let mut should_rebuild = false;
            {
                let package = self.packages.get(index).unwrap();
                if let Some(rebuild_time) = self.rebuild_interval {
                    if let Some(last_build) = package.last_built {
                        let elapsed = Utc::now().signed_duration_since(last_build);

                        if elapsed > Duration::try_seconds(rebuild_time as i64).unwrap() && package.status == PackageStatus::BUILT {
                            info!("Scheduled rebuild of package {}", package.get_package_name());
                            should_rebuild = true;
                        }
                    }
                }
                if package.status == PackageStatus::PENDING {
                    should_rebuild = true;
                }
            }
            if should_rebuild {
                self.dispatch_package(index).unwrap();
            }
        }
    }

    pub fn dispatch_package(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        let worker_id = self.get_next_free_worker_id();

        let package = self.packages.get_mut(index).unwrap();
        if package.status != PackageStatus::PENDING {
            package.set_status(PackageStatus::PENDING);
        }

        if let Some(worker_id) = worker_id {
            info!("Dispatch {} to worker {}", package.get_package_name(), worker_id);
            let worker = self.workers.get_mut(&worker_id).unwrap();
            worker.dispatch_package(package)?;
        }
        Ok(())
    }

    pub fn set_package_status(&mut self, package_name: &String, status: PackageStatus) {
        for package in self.packages.iter_mut() {
            if package.get_package_name().eq(package_name) {
                package.set_status(status);
                return;
            }
        }
    }

    pub fn rebuild_all_packages(&mut self)
    {
        for package in self.packages.iter_mut() {
            if package.status != PackageStatus::BUILDING {
                package.set_status(PackageStatus::PENDING);
            }
        }
    }

    pub fn set_package_build_data(
        &mut self,
        package_name: &String,
        time: Option<DateTime<Utc>>,
        version: Option<String>,
    ) {
        for package in self.packages.iter_mut() {
            if package.get_package_name().eq(package_name) {
                package.last_built = time;
                package.package.last_built_version = version;
                return;
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
                self.set_package_status(current_job, PackageStatus::PENDING);
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