extern crate core;

mod commands;
mod models;
mod orchestrator;
mod utils;
mod worker;
mod logs;
mod builder;

use std::fs::File;
use std::sync::Arc;
use std::time::Duration;
use log::{debug, error, info};
use simplelog::{ColorChoice, CombinedLogger, Config as SimpleLogConfig, TerminalMode, TermLogger, WriteLogger};
use tokio::sync::RwLock;
use tokio::time::sleep;
use crate::builder::bubblewrap::Bubblewrap;
use crate::models::config::Config;
use crate::orchestrator::websocket::WebsocketClient;
use crate::worker::State;

pub async fn start(config: &Config)
{
    let state = Arc::new(RwLock::new(State::from_config(config)));

    loop {
        let mut websocket_client = WebsocketClient::new(config, state.clone());
        let res = websocket_client.listen().await;

        error!("Lost connection to server: {:?}", res);
        error!("Retrying connection in 5 seconds ...");
        sleep(Duration::from_secs(5)).await;
    }
}


#[tokio::main]
async fn main() {
    let config = Config::new().await.unwrap();

    CombinedLogger::init(
        vec![
            TermLogger::new(config.log_level, SimpleLogConfig::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(config.log_level, SimpleLogConfig::default(), File::create(&config.log_path).unwrap()),
        ]
    ).unwrap();

    debug!("Loaded config {:#?}", config);

    info!("Starting aur-build-worker with version {}", env!("CARGO_PKG_VERSION"));

    let bubblewrap = Bubblewrap::from_config(&config);
    bubblewrap.create(config.force_base_sandbox_create).await.unwrap();

    // if let Some (package) = &config.package_build {
    //
    // }

    start(&config).await;
    info!("Worker terminated");
}