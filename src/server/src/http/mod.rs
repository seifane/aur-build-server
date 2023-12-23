mod websocket;
mod util;
mod methods;

use std::str::from_utf8;
use std::sync::{Arc};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{RwLock};
use warp::{Filter, reject, Rejection};
use warp::header::headers_cloned;
use warp::http::{HeaderMap, HeaderValue};
use common::http::payloads::PackageRebuildPayload;
use crate::http::methods::{get_logs, get_packages, get_workers, rebuild_packages, upload_package};
use crate::http::websocket::handle_websocket_connection;
use crate::models::config::Config;
use crate::orchestrator::Orchestrator;


async fn authorize((token, headers): (String, HeaderMap<HeaderValue>)) -> Result<(), Rejection> {
    match headers.get("authorization") {
        Some(authorization) => {
            let auth = from_utf8(authorization.as_bytes()).unwrap();
            if auth == token.as_str() {
                return Ok(());
            }
            return Err(reject::reject());
        }
        None => return Err(reject::reject()),
    }
}

pub fn with_auth(token: String) -> impl Filter<Extract=((), ), Error=Rejection> + Clone {
    headers_cloned()
        .map(move |headers: HeaderMap<HeaderValue>| (token.clone(), headers))
        .and_then(authorize)
}

pub async fn start_http(orchestrator: Arc<RwLock<Orchestrator>>, config: Config) {
    let next_id: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(1));

    let orchestrator_filter = warp::any().map(move || orchestrator.clone());
    let next_id_filter = warp::any().map(move || next_id.clone());
    let ws = warp::path("ws")
        .and(warp::ws())
        .and(orchestrator_filter.clone())
        .and(next_id_filter)
        .map(|ws: warp::ws::Ws, orchestrator, next_id: Arc<AtomicUsize>| {
            let id = next_id.fetch_add(1, Ordering::Relaxed);
            ws.on_upgrade(move |websocket| handle_websocket_connection(websocket, orchestrator, id))
        });

    let api_routes = warp::path("api");

    let get_packages = api_routes.and(warp::path("packages"))
        .and(with_auth(config.api_key.clone()))
        .untuple_one()
        .and(warp::get())
        .and(orchestrator_filter.clone())
        .and_then(move |o| get_packages(o));

    let get_workers = api_routes.and(warp::path("workers"))
        .and(with_auth(config.api_key.clone()))
        .untuple_one()
        .and(warp::get())
        .and(orchestrator_filter.clone())
        .and_then(move |o| get_workers(o));

    let get_logs = api_routes.and(warp::path("logs"))
        .and(with_auth(config.api_key.clone()))
        .untuple_one()
        .and(warp::path::param())
        .and(warp::path::param())
        .and(warp::get())
        .and_then(move |package: String, log_type: String| get_logs(package, log_type));

    let post_rebuild_packages = api_routes.and(warp::path("rebuild"))
        .and(with_auth(config.api_key.clone()))
        .untuple_one()
        .and(warp::post())
        .and(orchestrator_filter.clone())
        .and(warp::body::content_length_limit(1024 * 32))
        .and(warp::body::json())
        .and_then(move |o, payload: PackageRebuildPayload| rebuild_packages(o, payload));

    let upload_package = api_routes.and(warp::path("worker"))
        .and(warp::path("upload"))
        .and(with_auth(config.api_key.clone()))
        .untuple_one()
        .and(warp::post())
        .and(warp::multipart::form().max_length(1024 * 1000 * 1000 * 1000))
        .and(orchestrator_filter.clone())
        .and_then(move |f, o| upload_package(o, f));

    let files = warp::path("repo")
        .and(warp::get())
        .and(warp::fs::dir("serve"));

    let routes = warp::any().and(
        files
            .or(ws)
            .or(get_packages)
            .or(get_workers)
            .or(get_logs)
            .or(post_rebuild_packages)
            .or(upload_package)
    );

    warp::serve(routes).run(([0, 0, 0, 0], config.get_port())).await;
}