use anyhow::{bail, Result};
use log::warn;
use crate::builder::bubblewrap::Bubblewrap;
use crate::utils::sanitize_dependency;

pub async fn pacman_update(bubblewrap: &Bubblewrap) -> Result<()>
{
    let output = bubblewrap.run_sandbox(true, "base", "/", "pacman", vec!["-Syyu"], None, None)
        .await?;

    if !output.status.success() {
        bail!("Failed to update with code {:?}", output.status.code());
    }

    Ok(())
}

// TODO: Replace with proper call to libalpm
pub async fn is_package_in_repo(bubblewrap: &Bubblewrap, package_name: &String) -> bool {
    let output = bubblewrap.run_sandbox(true,"base", "/", "pacman", vec![
        "-Ss",
        format!("^{}$", sanitize_dependency(package_name)).as_str(),
    ], None, None).await;

    if output.is_err() {
        warn!("Could not check pacman for {}, {}", package_name, output.unwrap_err());
        return false;
    }
    output.unwrap().status.success()
}