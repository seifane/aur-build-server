mod http;
mod models;
mod utils;
mod orchestrator;

use std::fs::File;
use std::sync::Arc;
use simplelog::{ColorChoice, CombinedLogger, Config as SimpleLogConfig, TerminalMode, TermLogger, WriteLogger};
use clap::Parser;
use log::{debug, info};
use tokio::sync::{Mutex, RwLock};
use crate::http::start_http;
use crate::models::args::Args;
use crate::models::config::Config;
use crate::orchestrator::Orchestrator;
use crate::utils::repo::Repo;

pub async fn start(args: Args) {
    let config = Config::from_file(args.config_path);
    let orchestrator = Arc::new(RwLock::new(Orchestrator::from_config(&config)));
    let repo = Arc::new(Mutex::new(Repo::from_config(&config)));

    let res = repo.lock().await.add_packages_to_repo(Vec::new()).await;
    if let Err(err) = res {
        debug!("{:?}", err);
    }

    info!("Starting orchestrator");
    let orchestrator_task = tokio::task::spawn(Orchestrator::dispatch_loop(orchestrator.clone()));

    info!("Starting http");
    start_http(orchestrator, repo, config).await;
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
