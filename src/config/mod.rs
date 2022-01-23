use serde::{Deserialize};
use std::fs::File;
use std::io::{BufReader, Read};


#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub packages: Vec<String>
}

pub fn load_config() -> Config {
    let file = File::open("config/config.json").unwrap();

    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents).unwrap();

    let config: Config = serde_json::from_str(contents.as_str()).unwrap();
    config
}