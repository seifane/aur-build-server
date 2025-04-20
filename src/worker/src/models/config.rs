use anyhow::{Context, Result};
use std::path::PathBuf;
use clap::builder::TypedValueParser;

use clap::Parser;
use log::LevelFilter;
use serde::Deserialize;

#[derive(Deserialize, Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct SharedConfig {
    /// Path of the server configuration file. Default: './config_worker.json'
    #[clap(short, long)]
    #[serde(skip)]
    pub config_path: Option<PathBuf>,

    /// Log level. Default: 'info'
    #[arg(
        long,
        value_parser = clap::builder::PossibleValuesParser::new(["off", "error", "warn", "info", "debug", "trace"])
        .map(|s| s.parse::<log::LevelFilter>().unwrap()),
    )]
    pub log_level: Option<LevelFilter>,
    /// Log file output for the worker. Default: './aur-build-worker.log'
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    pub log_path: Option<PathBuf>,

    /// Path to the pacman configuration to use. Default: './config/pacman.conf'
    #[clap(short, long, value_hint = clap::ValueHint::DirPath)]
    pub pacman_config_path: Option<PathBuf>,
    /// Path to the pacman mirrorlist to use. Default: './config/mirrorlist'
    #[clap(short = 'm', long, value_hint = clap::ValueHint::DirPath)]
    pub pacman_mirrorlist_path: Option<PathBuf>,

    /// Path to the directory where packages will be cloned and built. Default: './worker/data'
    #[clap(short = 'd', long, value_hint = clap::ValueHint::DirPath)]
    pub data_path: Option<PathBuf>,
    /// Path to the directory where the sandbox will be stored. Default: './worker/sandbox'
    #[clap(short = 's', long, value_hint = clap::ValueHint::DirPath)]
    pub sandbox_path: Option<PathBuf>,
    /// Path to the directory where build logs will be stored. Default: './worker/logs'
    #[clap(short = 'l', long, value_hint = clap::ValueHint::DirPath)]
    pub build_logs_path: Option<PathBuf>,

    /// Base url to the server. Example: 'http://server:8888'
    #[clap(short = 'b', long)]
    pub base_url: Option<String>,
    /// Base websocket url to the server. Example: 'ws://server:8888'
    #[clap(short = 'w', long)]
    pub base_url_ws: Option<String>,
    /// API key to use for authentication
    #[clap(short = 'k', long)]
    pub api_key: Option<String>,

    /// Should the worker rebuild its sandbox from scratch at startup. Default 'false'
    #[clap(short = 'f', long)]
    pub force_base_sandbox_create: Option<bool>,

    /// Used for debugging, builds the given AUR package and then exits, available only through CLI args.
    #[clap(long)]
    pub package_build: Option<String>
}

impl SharedConfig {
    pub async fn try_from_file(path: &PathBuf) -> Result<SharedConfig>
    {
        let file = tokio::fs::read_to_string(path).await?;
        let config: SharedConfig = serde_json::from_str(file.as_str())?;
        Ok(config)
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub log_level: LevelFilter,
    pub log_path: PathBuf,

    pub pacman_config_path: PathBuf,
    pub pacman_mirrorlist_path: PathBuf,

    pub data_path: PathBuf,
    pub sandbox_path: PathBuf,
    pub build_logs_path: PathBuf,

    pub base_url: String,
    pub base_url_ws: String,
    pub api_key: String,

    pub force_base_sandbox_create: bool,
    // pub package_build: Option<String>
}

impl Config {
    pub async fn new() -> Result<Config> {
        let cli_config= SharedConfig::parse();
        let config_path = cli_config.config_path.clone().unwrap_or(PathBuf::from("./config_worker.json"));
        let file_config = SharedConfig::try_from_file(&config_path).await
            .with_context(|| format!("Failed to read config from file {:?}", config_path))?;

        let config = Config {
            log_level: cli_config.log_level.unwrap_or(LevelFilter::Info),
            log_path: cli_config.log_path.unwrap_or(file_config.log_path.unwrap_or(PathBuf::from("./aur-build-worker.log"))),

            pacman_config_path: cli_config.pacman_config_path.unwrap_or(file_config.pacman_config_path.unwrap_or(PathBuf::from("./config/pacman.conf"))),
            pacman_mirrorlist_path: cli_config.pacman_mirrorlist_path.unwrap_or(file_config.pacman_mirrorlist_path.unwrap_or(PathBuf::from("./config/mirrorlist"))),

            data_path: cli_config.data_path.unwrap_or(file_config.data_path.unwrap_or(PathBuf::from("./worker/data"))),
            sandbox_path: cli_config.sandbox_path.unwrap_or(file_config.sandbox_path.unwrap_or(PathBuf::from("./worker/sandbox"))),
            build_logs_path: cli_config.build_logs_path.unwrap_or(file_config.build_logs_path.unwrap_or(PathBuf::from("./worker/logs"))),

            base_url: cli_config.base_url.unwrap_or(file_config.base_url.unwrap()),
            base_url_ws: cli_config.base_url_ws.unwrap_or(file_config.base_url_ws.unwrap()),
            api_key: cli_config.api_key.unwrap_or(file_config.api_key.unwrap()),

            force_base_sandbox_create: cli_config.force_base_sandbox_create.unwrap_or(file_config.force_base_sandbox_create.unwrap_or(false)),
            // package_build: cli_config.package_build
        };

        Ok(config)
    }
}