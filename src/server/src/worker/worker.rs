use anyhow::Result;
use log::info;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use common::http::responses::WorkerResponse;
use common::models::{PackageStatus, WorkerStatus};
use common::messages::{WebsocketMessage};
use crate::models::server_package::{ServerPackage};

pub struct Worker {
    pub receiver_task: JoinHandle<()>,
    pub sender_task: JoinHandle<()>,
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



    pub fn dispatch_package(&mut self, package: &mut ServerPackage) -> Result<()>
    {
        self.sender.send(
            WebsocketMessage::JobSubmit {
                package: package.get_package_job(),
            }
        )?;

        self.status = WorkerStatus::DISPATCHED;
        package.set_status(PackageStatus::BUILDING);
        self.current_job = Some(package.get_package_name().clone());

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
        if !self.sender_task.is_finished() {
            self.sender_task.abort();
        }
        if !self.receiver_task.is_finished() {
            self.receiver_task.abort();
        }
    }

    pub fn to_http_response(&self) -> WorkerResponse {
        WorkerResponse {
            id: self.id,
            status: self.status,
            current_job: self.current_job.clone(),
            is_authenticated: self.is_authenticated,
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.terminate();
    }
}