use std::error::Error;
use log::info;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use common::models::{PackageStatus, WorkerStatus};
use common::messages::{WebsocketMessage};
use crate::models::package::{Package};

#[derive(Serialize)]
pub struct Worker {
    #[serde(skip_serializing)]
    pub receiver_task: JoinHandle<()>,
    #[serde(skip_serializing)]
    pub sender_task: JoinHandle<()>,
    #[serde(skip_serializing)]
    pub sender: UnboundedSender<WebsocketMessage>,

    id: usize,
    status: WorkerStatus,
    current_job: Option<String>,
    is_authenticated: bool
}

impl Worker {
    pub fn new(
        receiver_task: JoinHandle<()>,
        sender_task: JoinHandle<()>,
        sender: UnboundedSender<WebsocketMessage>,
        id: usize,
    ) -> Worker
    {
        Worker {
            receiver_task,
            sender_task,
            sender,
            id,
            status: WorkerStatus::UNKNOWN,
            current_job: None,
            is_authenticated: false,
        }
    }

    pub fn dispatch_package(&mut self, package: &mut Package) -> Result<(), Box<dyn Error>>
    {
        self.sender.send(
            WebsocketMessage::JobSubmit {
                package: package.name.clone(),
                run_before: package.run_before.clone(),
                last_built_version: package.last_built_version.clone(),
            }
        )?;

        self.status = WorkerStatus::DISPATCHED;
        package.set_status(PackageStatus::BUILDING);
        self.current_job = Some(package.name.clone());

        Ok(())
    }

    pub fn get_current_job(&self) -> &Option<String>
    {
        &self.current_job
    }

    pub fn set_current_job(&mut self, job: Option<String>)
    {
        self.current_job = job;
    }

    pub fn get_status(&self) -> WorkerStatus
    {
        self.status
    }

    pub fn set_status(&mut self, status: WorkerStatus)
    {
        info!("Worker {} status = {:?}", self.id, status);
        self.status = status;
    }

    pub fn is_authenticated(&self) -> bool
    {
        return self.is_authenticated;
    }

    pub fn authenticate(&mut self) {
        info!("Worker id {} authenticated successfully", self.id);
        self.is_authenticated = true;
    }

    pub fn terminate(&mut self)
    {
        info!("Terminating worker id {}", self.id);
        self.sender_task.abort();
        self.receiver_task.abort();
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.terminate();
    }
}