use anyhow::Result;
use std::path::PathBuf;
use log::info;
use tokio::fs::{create_dir_all, remove_dir_all};

pub async fn post_build_clean(data_path: &PathBuf) -> Result<()> {
    info!("Removing data directory ...");
    let _ = remove_dir_all(data_path).await;

    info!("Recreating data directory ...");
    create_dir_all(data_path).await?;
    create_dir_all(data_path.join("_built")).await?;
    Ok(())
}