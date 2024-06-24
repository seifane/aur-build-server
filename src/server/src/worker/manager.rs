use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;

use common::models::WorkerStatus;
use crate::models::config::Config;

use crate::models::server_package::ServerPackage;
use crate::worker::worker::Worker;

pub enum WorkerDispatchResult {
    NoneAvailable,
    Ok,
    Err(Box<dyn Error>)
}

pub struct WorkerManager {
    config: Arc<RwLock<Config>>,

    pub workers: HashMap<usize, Worker>
}

impl WorkerManager {
    pub fn new(config: Arc<RwLock<Config>>) -> Self
    {
        WorkerManager {
            config,

            workers: Default::default(),
        }
    }

    pub async fn try_authenticate_worker(&mut self, worker_id: &usize, submitted_key: String) -> bool {
        if let Some(worker) = self.workers.get_mut(worker_id) {
            if submitted_key == self.config.read().await.api_key {
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

    pub fn add(&mut self, id: usize, worker: Worker) {
        self.workers.insert(id, worker);
    }

    pub fn remove(&mut self, worker_id: usize) -> Option<Worker>
    {
        self.workers.remove(&worker_id)
    }

    pub fn dispatch(&mut self, package: &mut ServerPackage) -> WorkerDispatchResult {
        match self.get_next_free_worker_id() {
            None => WorkerDispatchResult::NoneAvailable,
            Some(id) => {
                match self.workers.get_mut(&id).unwrap().dispatch_package(package) {
                    Ok(_) => WorkerDispatchResult::Ok,
                    Err(e) => WorkerDispatchResult::Err(e)
                }
            }
        }
    }
}