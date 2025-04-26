use anyhow::Result;
use std::path::PathBuf;

use clap::builder::TypedValueParser;

use clap::Parser;
use log::LevelFilter;
use serde::Deserialize;

macro_rules! merge_config_option {
    ($a:expr, $b:expr, $f: ident) => {
        {
            match ($a.$f, $b.$f) {
                (Some(v), _) => Some(v),
                (None, Some(v)) => Some(v),
                _ => None
            }
        }
    };
}

#[derive(Deserialize, Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct SharedConfig {
    /// Path of the server configuration file. Default: './config_server.json'
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
    /// Log file output for the server. Default: './aur-build-server.log'
    #[clap(long, value_hint = clap::ValueHint::FilePath)]
    pub log_path: Option<PathBuf>,

    /// Sets the API Key that will be used by the workers and CLI to authenticate
    #[clap(short = 'k', long)]
    pub api_key: Option<String>,
    /// Port to listen on. Default: '8888'
    #[clap(short = 'p', long)]
    pub port: Option<u16>,

    /// Name of the Arch repo to create and serve
    #[clap(short = 'r', long)]
    pub repo_name: Option<String>,
    /// ID of the GPG key used to sign the packages
    #[clap(short = 's', long)]
    pub sign_key: Option<String>,
    /// The time in seconds between rebuild attempts
    #[clap(short = 't', long)]
    pub rebuild_time: Option<u64>,

    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    /// Path to store built packages and serve them. Default: './server/serve'
    pub serve_path: Option<PathBuf>,
    /// Path to store built packages and serve them. Default: './server/build_logs'
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    pub build_logs_path: Option<PathBuf>,
    /// Path to store database. Default: './server/aur-build.sqlite'
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    pub database_path: Option<PathBuf>,

    #[clap(skip)]
    pub webhooks: Option<Vec<String>>,
    /// Verify the validity of the presented ssl certificate. Default: 'true'
    #[clap(long)]
    pub webhook_verify_ssl: Option<bool>,
    /// Trust this certificate when sending webhooks. Must be a path to a valid .pem certificate.
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    pub webhook_certificate: Option<PathBuf>,

    #[clap(skip)]
    pub packages: Vec<LegacyPackageDefinition>,
}

impl SharedConfig {
    pub async fn try_from_file(path: &PathBuf) -> Result<SharedConfig>
    {
        let file = tokio::fs::read_to_string(path).await?;
        let config: SharedConfig = serde_json::from_str(file.as_str())?;
        Ok(config)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct LegacyPatch {
    pub url: String,
    pub sha512: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LegacyPackageDefinition {
    pub name: String,
    pub run_before: Option<String>,
    pub patches: Option<Vec<LegacyPatch>>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub log_level: LevelFilter,
    pub log_path: PathBuf,

    pub api_key: String,
    pub port: u16,

    pub repo_name: String,
    pub sign_key: Option<String>,
    pub rebuild_time: Option<u64>,

    pub serve_path: PathBuf,
    pub build_logs_path: PathBuf,
    pub database_path: PathBuf,

    pub webhooks: Vec<String>,
    pub webhook_verify_ssl: bool,
    pub webhook_certificate: Option<PathBuf>,
    pub packages: Vec<LegacyPackageDefinition>,
}

impl Config {
    pub async fn new() -> Result<Config> {
        let cli_config= SharedConfig::parse();
        let file_config = SharedConfig::try_from_file(
            &cli_config.config_path.clone().unwrap_or(PathBuf::from("./config_server.json"))
        ).await?;

        let config = Config {
            log_level: cli_config.log_level.unwrap_or(LevelFilter::Info),
            log_path: cli_config.log_path.unwrap_or(file_config.log_path.unwrap_or(PathBuf::from("./aur_build_server.log"))),

            api_key: cli_config.api_key.unwrap_or(file_config.api_key.unwrap()),
            port: cli_config.port.unwrap_or(file_config.port.unwrap_or(8888)),

            repo_name: cli_config.repo_name.unwrap_or(file_config.repo_name.unwrap_or(String::from("aurbuild"))),
            sign_key: merge_config_option!(cli_config, file_config, sign_key),
            rebuild_time: merge_config_option!(cli_config, file_config, rebuild_time),

            serve_path: cli_config.serve_path.unwrap_or(file_config.serve_path.unwrap_or(PathBuf::from("./server/serve"))),
            build_logs_path: cli_config.build_logs_path.unwrap_or(file_config.build_logs_path.unwrap_or(PathBuf::from("./server/build_logs"))),
            database_path: cli_config.database_path.unwrap_or(file_config.database_path.unwrap_or(PathBuf::from("./server/aur-build.sqlite"))),

            webhooks: cli_config.webhooks.unwrap_or(file_config.webhooks.unwrap_or_default()),
            webhook_verify_ssl: cli_config.webhook_verify_ssl.unwrap_or(file_config.webhook_verify_ssl.unwrap_or(true)),
            webhook_certificate: merge_config_option!(cli_config, file_config, webhook_certificate),
            packages: file_config.packages,
        };

        Ok(config)
    }
}