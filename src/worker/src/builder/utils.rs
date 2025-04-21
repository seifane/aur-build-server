use anyhow::Result;
use log::{error, info, trace};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs::{create_dir_all, remove_dir_all};
use tokio::process::{Child, Command};
use crate::logs::{write_tail_logs, LogSection};

pub async fn post_build_clean(data_path: &PathBuf) -> Result<()> {
    info!("Removing data directory ...");
    let _ = remove_dir_all(data_path).await;

    info!("Recreating data directory ...");
    create_dir_all(data_path).await?;
    create_dir_all(data_path.join("_built")).await?;
    Ok(())
}

pub fn run_command(mut command: Command, log_path: Option<&PathBuf>, log_section: Option<LogSection>) -> Result<Child> {
    let (reader, writer) = os_pipe::pipe()?;

    let child = command.stdin(Stdio::null())
        .stdout(writer.try_clone()?)
        .stderr(writer).spawn()?;

    let log_path = match log_path {
        None => None,
        Some(p) => Some(p.clone())
    };

    drop(command);

    tokio::task::spawn(async move {
        match write_tail_logs(reader, log_path, log_section).await {
            Ok(_) => trace!("Successfully wrote command logs"),
            Err(e) => error!("Failed to write command logs: {}", e),
        }
    });

    Ok(child)
}
