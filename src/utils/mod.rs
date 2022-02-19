use std::error::Error;
use std::process::{Command};
use std::string::String;
use ::log::LevelFilter;

pub mod file;
pub mod log;
pub mod package;
pub mod git;
pub mod pkgbuild;
pub mod aurweb;
pub mod package_data;
pub mod tree;

pub fn parse_log_level_from_string(level: String) -> LevelFilter {
    match level.as_str() {
        "off" => LevelFilter::Off,
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Debug
    }
}

pub fn build_repo(repo_name: String) -> Result<(), Box<dyn Error>> {
    Command::new("sh")
        .arg("-c")
        .arg(format!("cd serve; repo-add {}.db.tar.gz *.pkg.tar.zst", repo_name)).output().unwrap();

    Ok(())
}
