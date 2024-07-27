use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use common::messages::WebsocketMessage;
use common::models::PackageJob;
use crate::builder::Builder;
use crate::models::config::Config;
use crate::worker::State;

pub type WebsocketReceiveStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
pub type WebsocketSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

async fn websocket_send_task(mut tx_ws: WebsocketSink, mut rx: UnboundedReceiverStream<WebsocketMessage>)
{
    while let Some(message) = rx.next().await {
        let res = tx_ws.send(Message::Text(serde_json::to_string(&message).unwrap())).await;
        if let Err(res) = res {
            error!("Error while sending message {}", res);
        }
    }
}


pub async fn websocket_recv_task(mut ws_rx: WebsocketReceiveStream, worker: Arc<RwLock<State>>)
{
    while let Some(message) = ws_rx.next().await {
        if let Ok(message) = message {
            if message.is_text() {
                let payload = message.to_string();
                let parsed: WebsocketMessage = serde_json::from_str(payload.as_str()).unwrap();

                let res = handle_message(&parsed, &worker).await;

                if let Err(e) = res {
                    error!("Failed to handle message: {}", e);
                }
            }
        }
    }
}

async fn handle_job_submit(package_job: &PackageJob, state: &Arc<RwLock<State>>) -> Result<()> {
    if state.read().await.current_job.is_some() {
        return Ok(());
    }
    if let Some(handle) = state.read().await.monitor_handle.as_ref() {
        if !handle.is_finished() {
            handle.abort();
        }
    }

    state.write().await.current_job = Some(package_job.clone());

    let cloned_state = state.clone();
    let job = package_job.clone();
    let http_client = state.read().await.get_http_client();
    let config = state.read().await.config.clone();

    let monitor_handle = tokio::task::spawn(async move {
        let (tx, mut rx) = mpsc::channel(1);

        let builder = Builder::new(
            tx,
            http_client,
            job,
            &config
        );
        let handle = tokio::task::spawn(async move {
            builder.process_package().await
        });

        while let Some(msg) = rx.recv().await {
            {
                let _ = cloned_state.write().await.set_status(msg); // TODO: handle ?
            }
        }
        info!("State receiver was closed");

        if !handle.is_finished() {
            handle.abort();
            info!("Builder task aborted");
        } else {
            let res = handle.await;
            match res {
                Ok(res) => {
                    info!("Builder task finished with {:?}", res);
                }
                Err(e) => {
                    error!("Failed to join on builder thread: {}", e);
                }
            }
        }

        let _ = cloned_state.write().await.clear_job(); // TODO: handle ?
    });

    state.write().await.monitor_handle = Some(monitor_handle);

    Ok(())
}

async fn handle_message(message: &WebsocketMessage, state: &Arc<RwLock<State>>) -> Result<()> {
    match message {
        WebsocketMessage::JobSubmit { package} => {
            handle_job_submit(package, state).await?;
        }
        WebsocketMessage::WorkerStatusRequest { .. } => {
            state.write().await.push_state()?;
        }
        _ => {}
    }

    Ok(())
}


pub struct WebsocketClient {
    url: String,
    api_key: String,

    pub state: Arc<RwLock<State>>,
}

impl WebsocketClient {
    pub fn new(config: &Config, state: Arc<RwLock<State>>) -> WebsocketClient
    {
        WebsocketClient {
            url: format!("{}/ws", config.base_url_ws),
            api_key: config.api_key.clone(),

            state,
        }
    }

    pub async fn connect(&mut self) -> Result<(JoinHandle<()>, JoinHandle<()>)> {
        info!("Connecting to websocket");
        let (ws_stream, _) = connect_async(&self.url).await.with_context(|| "Failed to connect")?;
        info!("Connected to websocket");

        let (tx_ws, rx_ws) = ws_stream.split();
        let (tx, rx) = mpsc::unbounded_channel();

        let send_task = tokio::task::spawn(websocket_send_task(tx_ws, UnboundedReceiverStream::new(rx)));
        let recv_task = tokio::task::spawn(websocket_recv_task(rx_ws, self.state.clone()));

        info!("Sending authentication ...");
        tx.send(WebsocketMessage::Authenticate {
            api_key: self.api_key.clone(),
        }).with_context(|| "Failed to send authenticate message")?;

        self.state.write().await.sender = Some(tx);

        Ok((send_task, recv_task))
    }

    pub async fn listen(&mut self) -> Result<()>
    {
        let (send_task, recv_task) = self.connect().await?;

        loop {
            if send_task.is_finished() || recv_task.is_finished() {
                warn!("Websocket connection closed");
                self.state.write().await.sender = None;
                break;
            }
            sleep(Duration::from_millis(1000)).await;
        }

        Ok(())
    }
}

