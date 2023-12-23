use std::fs::File;
use std::io::{BufReader, Read};
use serde::{Deserialize};

#[derive(Deserialize, Debug, Clone)]
pub struct PackageConfig {
    pub name: String,
    pub run_before: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub repo_name: String,
    pub sign: Option<bool>,

    pub api_key: String,
    pub rebuild_time: Option<u64>,
    pub packages: Vec<PackageConfig>,

    port: Option<u16>,
}

impl Config {
    pub fn from_file(path: String) -> Config {
        let file = File::open(path).unwrap();

        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents).unwrap();

        serde_json::from_str(contents.as_str()).unwrap()
    }

    pub fn get_port(&self) -> u16
    {
        self.port.unwrap_or(8888)
    }
}