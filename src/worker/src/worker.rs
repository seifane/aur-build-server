use anyhow::{Context, Result};
use tokio::sync::mpsc::{UnboundedSender};
use tokio::task::JoinHandle;
use common::messages::{WebsocketMessage};
use common::models::{PackageJob, WorkerStatus};
use crate::models::config::Config;
use crate::orchestrator::http::HttpClient;

pub struct State {
    pub config: Config,

    pub current_job: Option<PackageJob>,
    pub monitor_handle: Option<JoinHandle<()>>,
    pub status: WorkerStatus,

    pub sender: Option<UnboundedSender<WebsocketMessage>>,
}

impl State {
    pub fn from_config(config: &Config) -> State {
        State {
            config: config.clone(),
            current_job: None,
            monitor_handle: None,
            status: WorkerStatus::STANDBY,

            sender: None
        }
    }

    pub fn clear_job(&mut self) -> Result<()>
    {
        self.status = WorkerStatus::STANDBY;
        self.current_job = None;
        self.push_state()
    }

    pub fn set_status(&mut self, status: WorkerStatus) -> Result<()>
    {
        self.status = status;
        self.push_state()
    }

    pub fn push_state(&self) -> Result<()>
    {
        if let Some(sender) = self.sender.as_ref() {
            let mut package = None;

            if let Some(job) = self.current_job.as_ref() {
                package = Some(job.definition.name.clone());
            }

            sender.send(WebsocketMessage::WorkerStatusUpdate {
                status: self.status.clone(),
                package
            }).with_context(|| "Failed to send message via sender".to_string())?;
        }
        Ok(())
    }

    pub fn get_http_client(&self) -> HttpClient
    {
        HttpClient::from_config(&self.config)
    }
}