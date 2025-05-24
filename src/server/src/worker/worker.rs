use actix_ws::{AggregatedMessage, AggregatedMessageStream, ProtocolError, Session};
use anyhow::{anyhow, Result};
use common::http::responses::WorkerResponse;
use common::messages::WebsocketMessage;
use common::models::{PackageJob, WorkerStatus};
use futures_util::{StreamExt};
use log::{debug, error, info};
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct InnerWorker {
    id: usize,
    status: WorkerStatus,
    current_job: Option<PackageJob>,
    version: String,
}

pub struct Worker {
    id: usize,
    inner: Arc<Mutex<InnerWorker>>,

    websocket_task: JoinHandle<()>,
    tx_message: UnboundedSender<WebsocketMessage>,
}

impl Worker {
    pub fn new(
        id: usize,
        session: Session,
        stream: AggregatedMessageStream,
    ) -> Worker {
        let inner = Arc::new(Mutex::new(InnerWorker {
            id,
            status: WorkerStatus::STANDBY,
            current_job: None,
            version: "Unknown".to_string(),
        }));

        let (tx_message, rx_message) = unbounded_channel();
        let websocket_task = actix_web::rt::spawn(websocket_loop(
            session,
            stream,
            rx_message,
            inner.clone(),
        ));

        Worker {
            id,
            inner,

            websocket_task,
            tx_message,
        }
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub async fn dispatch_package(&mut self, package_job: PackageJob) -> Result<()> {
        {
            let mut inner_lock = self.inner.lock().await;
            inner_lock.status = WorkerStatus::DISPATCHED;
            inner_lock.current_job = Some(package_job.clone());
        }

        self.tx_message.send(WebsocketMessage::JobSubmit {
            package: package_job,
        })?;
        Ok(())
    }

    pub async fn get_current_job(&self) -> Option<PackageJob> {
        self.inner.lock().await.current_job.clone()
    }

    pub async fn get_status(&self) -> WorkerStatus {
        self.inner.lock().await.status
    }

    pub fn is_finished(&self) -> bool {
        self.websocket_task.is_finished()
    }

    pub fn terminate(&mut self) {
        if !self.websocket_task.is_finished() {
            self.websocket_task.abort();
        }
    }

    pub async fn to_http_response(&self) -> WorkerResponse {
        let inner_lock = self.inner.lock().await;
        WorkerResponse {
            id: inner_lock.id,
            status: inner_lock.status,
            current_job: inner_lock
                .current_job
                .as_ref()
                .map(|i| i.definition.name.clone()),
            version: inner_lock.version.clone(),
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.terminate();
    }
}

async fn websocket_loop(
    mut session: Session,
    mut stream: AggregatedMessageStream,
    mut rx: UnboundedReceiver<WebsocketMessage>,
    state: Arc<Mutex<InnerWorker>>,
) {
    loop {
        tokio::select! {
            send_message = rx.recv() => {
                if let Some(message) = send_message {
                    if let Err(e) = session.text(
                        serde_json::to_string(&message).unwrap()
                    ).await {
                        error!("Error sending message to websocket: {}", e);
                        return;
                    }
                }
            }
            receive_message = stream.next() => {
                match receive_message {
                    None => {
                        error!("Websocket closed unexpectedly");
                        return;
                    }
                    Some(message) => {
                        if let Err(e) =
                            handle_received_message(message, &mut session, &state).await {
                            error!("Error handling received message: {}", e);
                            return;
                        }
                    }
                }
            }
        }
    }
}

async fn handle_received_message(
    received_message: Result<AggregatedMessage, ProtocolError>,
    session: &mut Session,
    state: &Arc<Mutex<InnerWorker>>,
) -> Result<()> {
    match received_message {
        Ok(message) => {
            match message {
                AggregatedMessage::Text(message) => {
                    let parsed: WebsocketMessage = serde_json::from_str(&message)?;
                    match parsed {
                        WebsocketMessage::WorkerHello {status, version} => {
                            let mut state = state.lock().await;
                            state.version = version;
                            state.status = status;
                            info!("Worker id {} connected successfully with version {}", state.id, state.version);
                        },
                        WebsocketMessage::WorkerStatusUpdate {status, job} => {
                            let mut state = state.lock().await;
                            state.status = status;
                            state.current_job = job;
                        }
                        _ => {}
                    }
                }
                AggregatedMessage::Binary(_) => debug!("Ignored WS binary"),
                AggregatedMessage::Ping(msg) => session.pong(&msg).await?,
                AggregatedMessage::Pong(_) => debug!("Received pong"),
                AggregatedMessage::Close(reason) => {
                    return Err(anyhow!("Websocket closed connection {:?}", reason));
                }
            }
        }
        Err(e) => {
            return Err(anyhow!("Error receiving message from websocket: {}", e));
        }
    }
    Ok(())
}
