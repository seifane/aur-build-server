use std::{fs, io};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use crate::utils::file::read_file_to_string;

pub fn write_logs(repo_name: &str, data: &[u8], suffix: &str) -> Result<(), io::Error> {
    let log_dir_path = Path::new("logs");
    if !log_dir_path.exists() {
        fs::create_dir(log_dir_path).unwrap();
    }
    let path = format!("logs/{}_{}.log", repo_name, suffix).to_string();
    let log_path = Path::new(&path);
    if log_path.exists() {
        fs::remove_file(log_path)?;
    }
    let mut file = File::create(log_path).unwrap();
    file.write_all(data)?;
    Ok(())
}

pub fn read_log(repo_name: &str, suffix: &str) -> Result<String, io::Error> {
    let path = format!("logs/{}_{}.log", repo_name, suffix).to_string();
    read_file_to_string(path.as_str())
}