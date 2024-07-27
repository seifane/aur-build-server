mod http;
mod models;
mod orchestrator;
mod webhooks;
mod repository;
mod worker;

use std::fs::File;
use std::sync::Arc;
use simplelog::{ColorChoice, CombinedLogger, Config as SimpleLogConfig, TerminalMode, TermLogger, WriteLogger};
use log::{debug, info};
use tokio::sync::{RwLock};
use crate::http::start_http;
use crate::models::config::Config;
use crate::orchestrator::Orchestrator;

pub async fn start(config: Config) {
    let config = Arc::new(RwLock::new(config));
    let orchestrator = Orchestrator::new(config.clone()).await;
    let orchestrator = Arc::new(RwLock::new(orchestrator));

    info!("Starting orchestrator");
    orchestrator.write().await.restore_state().await;
    let orchestrator_task = tokio::task::spawn(Orchestrator::dispatch_loop(orchestrator.clone()));

    info!("Starting http");
    start_http(orchestrator, config).await;
    info!("Stopped http");

    orchestrator_task.abort();
    info!("Stopped orchestrator");
}

#[tokio::main]
async fn main(){
    let config = Config::new().await.unwrap();

    CombinedLogger::init(
        vec![
            TermLogger::new(config.log_level, SimpleLogConfig::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(config.log_level, SimpleLogConfig::default(), File::create(&config.log_path).unwrap()),
        ]
    ).unwrap();

    debug!("Loaded config: {:#?}", config);

    info!("Starting aur-build-server with version {}", env!("CARGO_PKG_VERSION"));

    start(config).await;
}
