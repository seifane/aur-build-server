use std::fs::File;
use std::io::{BufReader, Read};
use serde::{Deserialize};
use common::models::{PackageDefinition};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub repo_name: String,
    pub sign_key: Option<String>,

    pub api_key: String,
    pub rebuild_time: Option<u64>,
    pub packages: Vec<PackageDefinition>,

    serve_path: Option<String>,

    port: Option<u16>,

    pub webhooks: Option<Vec<String>>,
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
    
    pub fn get_serve_path(&self) -> String
    {
        let path = self.serve_path.clone().unwrap_or("serve/".to_string());
        path.trim().trim_end_matches("/").to_string()
    }
}