use std::sync::Arc;
use log::info;
use tokio::sync::mpsc::{UnboundedSender};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::Mutex;
use common::messages::{WebsocketMessage};
use common::models::{Package, WorkerStatus};
use crate::orchestrator::http::HttpClient;


pub struct Worker {
    sender: UnboundedSender<WebsocketMessage>,
    status: WorkerStatus,
    pub current_package: Option<Package>,
    http_client: Arc<Mutex<HttpClient>>,
}

impl Worker {
    pub fn new(
        sender: UnboundedSender<WebsocketMessage>,
        base_url: &String,
        api_key: &String
    ) -> Worker {
        Worker {
            sender,
            status: WorkerStatus::STANDBY,
            current_package: None,
            http_client: Arc::new(Mutex::new(HttpClient::new(base_url.clone(), api_key.clone()))),
        }
    }

    pub fn set_current_package(&mut self, current_package: Option<Package>) {
        self.current_package = current_package;
    }

    pub fn set_state(&mut self, status: WorkerStatus) -> Result<(), SendError<WebsocketMessage>> {
        self.status = status;
        self.push_state()
    }

    pub fn push_state(&mut self) -> Result<(), SendError<WebsocketMessage>> {
        let package = match &self.current_package {
            None => None,
            Some(current_package) => Some(current_package.name.clone())
        };

        let payload = WebsocketMessage::WorkerStatusUpdate {
            status: self.status,
            package,
        };

        info!("Status = {:?}", payload);

        self.sender.send(payload)
    }

    pub fn get_http_client(&self) -> Arc<Mutex<HttpClient>>
    {
        self.http_client.clone()
    }
}
