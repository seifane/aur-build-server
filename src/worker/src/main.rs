extern crate core;

mod commands;
mod errors;
mod models;
mod orchestrator;
mod utils;
mod worker;
mod logs;
mod builder;

use std::error::Error;
use std::fs::File;
use std::sync::Arc;
use clap::Parser;
use log::{info};
use simplelog::{ColorChoice, CombinedLogger, Config as SimpleLogConfig, TerminalMode, TermLogger, WriteLogger};
use tokio::sync::RwLock;
use crate::models::{Args, Config};
use crate::orchestrator::websocket::{connect, websocket_recv_task};
use crate::worker::Worker;

pub async fn start_worker(config: &Config) -> Result<(), Box<dyn Error>>
{
    let websocket = connect(format!("{}/ws", config.base_url_ws), &config.api_key).await;
    let worker = Arc::new(RwLock::new(
        Worker::new(websocket.1, &config.base_url, &config.api_key
        )));

    websocket_recv_task(websocket.2, worker.clone()).await;
    Ok(())
}


#[tokio::main]
async fn main() {
    let args: Args = Args::parse();

    CombinedLogger::init(
        vec![
            TermLogger::new(args.log_level, SimpleLogConfig::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(args.log_level, SimpleLogConfig::default(), File::create(args.log_path).unwrap()),
        ]
    ).unwrap();

    info!("Starting aur-build-worker with version {}", env!("CARGO_PKG_VERSION"));

    let config = Config::try_from_file(args.config_path).await.expect("Error while trying to open config file");

    start_worker(&config).await.expect("Worker general error");
    info!("Worker terminated");
}