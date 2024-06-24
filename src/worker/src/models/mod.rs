use std::error::Error;
use serde::Deserialize;
use clap::Parser;
use log::LevelFilter;
use clap::builder::TypedValueParser;
use serde_json::Value;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub base_url_ws: String,
    pub api_key: String,
}
impl Config {
    pub async fn try_from_file(path: String) -> Result<Config, Box<dyn Error>>
    {
        let file = tokio::fs::read_to_string(path).await?;
        let config: Config = serde_json::from_str(file.as_str())?;
        Ok(config)
    }
}

#[derive(Clone)]
pub struct PackageBuild {
    pub built: bool,
    pub version: String,
    pub additional_packages: Vec<String>
}

impl PackageBuild {
    pub fn new(built: bool, version: String, additional_packages: Vec<String>) -> PackageBuild {
        PackageBuild {
            built,
            version,
            additional_packages,
        }
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long, default_value_t = String::from("config_worker.json"))]
    pub config_path: String,

    #[arg(
    long,
    default_value = "info",
    value_parser = clap::builder::PossibleValuesParser::new(["off", "error", "warn", "info", "debug", "trace"])
    .map(|s| s.parse::<log::LevelFilter>().unwrap()),
    )]
    pub log_level: LevelFilter,

    #[clap(short = 'l', long, default_value_t = String::from("aur-build-worker.log"))]
    pub log_path: String
}