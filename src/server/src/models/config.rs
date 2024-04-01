use std::fs::File;
use std::io::{BufReader, Read};
use serde::{Deserialize};
use common::models::{Package, PackagePatch};

#[derive(Deserialize, Debug, Clone)]
pub struct PackageConfig {
    pub name: String,
    pub run_before: Option<String>,
    pub patches: Option<Vec<PackagePatch>>
}

impl PackageConfig {
    pub fn to_package(&self) -> Package {
        Package {
            name: self.name.clone(),
            run_before: self.run_before.clone(),
            patches: self.patches.clone().unwrap_or_default().clone(),
            last_built_version: None,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub repo_name: String,
    pub sign_key: Option<String>,

    pub api_key: String,
    pub rebuild_time: Option<u64>,
    pub packages: Vec<PackageConfig>,

    pub serve_path: Option<String>,

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
    
    pub fn get_serve_path(&self) -> String
    {
        let path = self.serve_path.clone().unwrap_or("serve/".to_string());
        path.trim().trim_end_matches("/").to_string()
    }
}