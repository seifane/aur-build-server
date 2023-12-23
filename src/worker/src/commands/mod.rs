pub mod git;
pub mod makepkg;
pub mod pacman;

use std::{fs, io};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn write_logs(repo_name: &str, data: &[u8], suffix: &str) -> Result<(), io::Error> {
    let log_dir_path = Path::new("worker_logs");
    if !log_dir_path.exists() {
        fs::create_dir(log_dir_path).unwrap();
    }
    let path = format!("worker_logs/{}_{}.log", repo_name, suffix).to_string();
    let log_path = Path::new(&path);
    if log_path.exists() {
        fs::remove_file(log_path)?;
    }
    let mut file = File::create(log_path).unwrap();
    file.write_all(data)?;
    Ok(())
}
