use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Output;
use log::{info};
use srcinfo::Srcinfo;
use tokio::process::Command;
use crate::builder::bubblewrap::Bubblewrap;

pub async fn get_src_info(data_path: &PathBuf, package_name: &String) -> Result<Srcinfo>
{
    let output = Command::new("makepkg")
        .arg("--printsrcinfo")
        .current_dir(data_path.join(package_name))
        .output()
        .await
        .with_context(|| format!("Failed to execute makepkg printsrcinfo for {}", package_name))?;

    Ok(
        Srcinfo::parse_buf(output.stdout.as_slice()).with_context(|| "Failed to parse SrcInfo")?
    )
}

pub async fn run_makepkg(bubblewrap: &Bubblewrap, package_name: &String) -> Result<Output>
{
    info!("Running makepkg for {}", package_name);

    let output = bubblewrap.run_sandbox(false, "current", "/package", "makepkg", vec![
        "--clean",
        "--noconfirm",
    ]).await?;
    Ok(output)
}

pub async fn get_package_version(data_path: &PathBuf, package_base: &String) -> Result<String> {
    let src_info = get_src_info(data_path, package_base).await?;

    let mut version = src_info.base.pkgver.clone();
    version += src_info.base.pkgrel.as_str();
    version += src_info.base.epoch.as_ref().unwrap_or(&"".to_string()).as_str();

    Ok(version)
}