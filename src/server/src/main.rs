mod http;
mod models;
mod orchestrator;
mod webhooks;
mod repository;
mod worker;

use std::fs::File;
use std::sync::Arc;
use simplelog::{ColorChoice, CombinedLogger, Config as SimpleLogConfig, TerminalMode, TermLogger, WriteLogger};
use clap::Parser;
use log::{info};
use tokio::sync::{RwLock};
use crate::http::start_http;
use crate::models::args::Args;
use crate::models::config::Config;
use crate::orchestrator::Orchestrator;

pub async fn start(args: Args) {
    let config = Config::from_file(args.config_path);

    let orchestrator = Orchestrator::new(&config).await;
    let orchestrator = Arc::new(RwLock::new(orchestrator));

    // orchestrator.write().await.restore_state().await;

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
    let args: Args = Args::parse();

    CombinedLogger::init(
        vec![
            TermLogger::new(args.log_level, SimpleLogConfig::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(args.log_level, SimpleLogConfig::default(), File::create(&args.log_path).unwrap()),
        ]
    ).unwrap();

    info!("Starting aur-build-server with version {}", env!("CARGO_PKG_VERSION"));

    start(args).await;
}
