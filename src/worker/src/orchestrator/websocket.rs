use std::sync::Arc;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio::sync::mpsc::{UnboundedSender};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use common::messages::WebsocketMessage;
use crate::builder::process_package;
use crate::worker::Worker;

pub type WebsocketReceiveStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
pub type WebsocketSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

async fn send_task(mut tx_ws: WebsocketSink, mut rx: UnboundedReceiverStream<WebsocketMessage>)
{
    while let Some(message) = rx.next().await {
        let res = tx_ws.send(Message::Text(serde_json::to_string(&message).unwrap())).await;
        if let Err(res) = res {
            error!("Error while sending message {}", res);
        }
    }
}

pub async fn connect(url: String, api_key: &String) -> (JoinHandle<()>, UnboundedSender<WebsocketMessage>, WebsocketReceiveStream) {
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    info!("Connected to websocket");

    let (tx_ws, rx_ws) = ws_stream.split();
    let (tx, rx) = mpsc::unbounded_channel();

    let send_task = tokio::task::spawn(send_task(tx_ws, UnboundedReceiverStream::new(rx)));

    tx.send(WebsocketMessage::Authenticate {
        api_key: api_key.clone(),
    }).expect("Failed to send authentication");

    (send_task, tx, rx_ws)
}

async fn handle_message(message: &WebsocketMessage, worker: Arc<RwLock<Worker>>) {
    match message {
        WebsocketMessage::JobSubmit { package} => {
            if worker.read().await.current_package.is_some() {
                return;
            }
            worker.write().await.set_current_package(Some(package.clone()));
            tokio::task::spawn(process_package(worker.clone()));
        }
        WebsocketMessage::WorkerStatusRequest { .. } => {
            worker.write().await.push_state().unwrap();
        }
        _ => {}
    }
}

pub async fn websocket_recv_task(mut ws_rx: WebsocketReceiveStream, worker: Arc<RwLock<Worker>>)
{
    while let Some(message) = ws_rx.next().await {
        if let Ok(message) = message {
            if message.is_text() {
                let payload = message.to_string();
                let parsed: WebsocketMessage = serde_json::from_str(payload.as_str()).unwrap();
                handle_message(&parsed, worker.clone()).await;
            }
        }
    }
}