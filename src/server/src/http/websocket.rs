use std::sync::{Arc};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use common::messages::{WebsocketMessage};
use crate::orchestrator::Orchestrator;
use crate::worker::worker::Worker;

pub async fn handle_websocket_connection(websocket: WebSocket, orchestrator: Arc<RwLock<Orchestrator>>, id: usize) {
    let (ws_tx, ws_rx) = websocket.split();
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);

    let receiver_task = tokio::task::spawn(websocket_recv_loop(ws_rx, orchestrator.clone(), id));
    let sender_task = tokio::task::spawn(websocket_send_loop(rx, ws_tx));

    tx.send(WebsocketMessage::WorkerStatusRequest {}).unwrap();


    // TODO: Move next id handling to worker manager
    let worker = Worker::new(receiver_task, sender_task, tx, id);

    orchestrator.write().await.worker_manager.add(id, worker);
}

async fn websocket_recv_loop(mut ws_rx: SplitStream<WebSocket>, orchestrator: Arc<RwLock<Orchestrator>>, id: usize) {
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) =>
                parse_websocket_message(msg, orchestrator.clone(), id).await,
            Err(e) => {
                error!("Websocket error {:?}", e);
                orchestrator.write().await.remove_worker(id).await;
                return;
            }
        };
    }
}

async fn websocket_send_loop(mut rx: UnboundedReceiverStream<WebsocketMessage>, mut ws_tx: SplitSink<WebSocket, Message>)
{
    while let Some(message) = rx.next().await {
        let payload = Message::text(
            serde_json::to_string(&message).unwrap()
        );

        let res = ws_tx.send(payload).await;
        if res.is_err() {
            rx.close();
            return;
        }
    }
}

async fn parse_websocket_message(message: Message, orchestrator: Arc<RwLock<Orchestrator>>, id: usize) {
    if message.is_close() {
        info!("Worker closed connection");
        orchestrator.write().await.remove_worker(id).await;
    } else if message.is_text() {
        let body = message.to_str().unwrap();
        let parsed: WebsocketMessage = serde_json::from_str(body).unwrap();
        handle_websocket_message(parsed, orchestrator, id).await;
    }
}

async fn handle_websocket_message(message: WebsocketMessage, orchestrator: Arc<RwLock<Orchestrator>>, id: usize) {
    match message {
        WebsocketMessage::WorkerStatusUpdate { status, job: package } => {
            orchestrator.write().await.worker_manager.set_worker_status(id, status, package);
        },
        WebsocketMessage::Authenticate {api_key} => {
            let is_authed = orchestrator.write().await.worker_manager.try_authenticate_worker(&id, api_key).await;
            if !is_authed {
                warn!("Failed to auth worker id {}", id);
                orchestrator.write().await.remove_worker(id).await;
            } else {
                info!("Requesting worker {} status", id);
                orchestrator.write().await.worker_manager.workers.get(&id).unwrap().sender.send(WebsocketMessage::WorkerStatusRequest {}).unwrap();
            }
        },
        _ => {}
    }
}

