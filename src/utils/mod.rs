use std::error::Error;
use std::process::{Command};
use std::string::String;

pub mod file;
pub mod log;
pub mod package;
pub mod git;
pub mod pkgbuild;

pub fn build_repo(repo_name: String) -> Result<(), Box<dyn Error>> {
    Command::new("sh")
        .arg("-c")
        .arg(format!("cd serve; repo-add {}.db.tar.gz *.pkg.tar.zst", repo_name)).output().unwrap();

    Ok(())
}
