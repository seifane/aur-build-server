use crate::models::config::Config;
use actix_ws::{AggregatedMessageStream, Session};
use common::models::{PackageJob, WorkerStatus};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::worker::worker::Worker;

pub enum WorkerDispatchResult {
    NoneAvailable,
    Ok,
    Err(anyhow::Error)
}

pub struct WorkerManager {
    config: Arc<RwLock<Config>>,
    next_id: Arc<AtomicUsize>,

    workers: Vec<Worker>
}

impl WorkerManager {
    pub fn new(config: Arc<RwLock<Config>>) -> Self
    {
        WorkerManager {
            config,
            next_id: Arc::new(AtomicUsize::new(0)),

            workers: Default::default(),
        }
    }

    pub fn get_workers(&self) -> &Vec<Worker>
    {
        &self.workers
    }

    pub async fn add(&mut self, session: Session, stream: AggregatedMessageStream) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let worker = Worker::new(
            id,
            session,
            stream,
            self.config.read().await.api_key.clone()
        );
        self.workers.push(worker);
    }

    pub fn remove(&mut self, worker_id: usize) -> Option<Worker>
    {
        if let Some(index) = self.workers.iter().position(|i| i.get_id() == worker_id) {
            return Some(self.workers.remove(index));
        }
        None
    }

    pub fn remove_finished_workers(&mut self) -> Vec<Worker> {
        // TODO: Replace with extract_if when stable
        let indexes: Vec<usize> = self.workers
            .iter()
            .enumerate()
            .filter(|(_, w)| w.is_finished())
            .map(|(i, _)| i)
            .rev()
            .collect();

        let mut removed = Vec::new();
        for index in indexes {
            removed.push(self.workers.remove(index));
        }

        removed
    }

    async fn get_next_free_worker(&mut self) -> Option<&mut Worker> {
        for worker in self.workers.iter_mut() {
            if worker.get_status().await == WorkerStatus::STANDBY && worker.is_authenticated().await {
                return Some(worker);
            }
        }
        None
    }

    pub async fn dispatch(&mut self, package_job: PackageJob) -> WorkerDispatchResult {
        match self.get_next_free_worker().await {
            None => WorkerDispatchResult::NoneAvailable,
            Some(worker) => {
                match worker.dispatch_package(package_job).await {
                    Ok(_) => WorkerDispatchResult::Ok,
                    Err(e) => WorkerDispatchResult::Err(e)
                }
            }
        }
    }
}