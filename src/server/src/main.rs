mod http;
mod models;
mod utils;
mod orchestrator;

use std::fs::File;
use std::sync::Arc;
use simplelog::{ColorChoice, CombinedLogger, Config as SimpleLogConfig, TerminalMode, TermLogger, WriteLogger};
use clap::Parser;
use log::{debug, error, info, warn};
use tokio::sync::{Mutex, RwLock};
use crate::http::start_http;
use crate::models::args::Args;
use crate::models::config::Config;
use crate::orchestrator::Orchestrator;
use crate::utils::repo::Repo;

pub async fn start(args: Args) {
    let config = Config::from_file(args.config_path);
    let orchestrator = Arc::new(RwLock::new(Orchestrator::from_config(&config)));

    let res = orchestrator.write().await.state.restore();
    let state_package_files = match res {
        Ok(_) =>  {
            info!("Restored state for packages");
            let mut package_files = Vec::new();
            for (_, package) in orchestrator.read().await.state.get_packages().iter() {
                package_files.append(&mut package.state.files.clone())
            }
            package_files
        },
        Err(e) => {
            error!("Failed to restore state for packages: {:?}", e);
            Vec::new()
        }
    };
    debug!("Packages restored from state {:?}", state_package_files);

    let repo = Arc::new(Mutex::new(Repo::from_config(&config)));
    if let Err(err) = repo.lock().await.set_repo_packages(state_package_files).await {
        warn!("Error while setting repo packages from state: {:?}", err);
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
