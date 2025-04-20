use anyhow::{bail, Result};
use log::{error, warn};
use crate::builder::bubblewrap::Bubblewrap;

pub async fn pacman_update_repos(bubblewrap: &Bubblewrap) -> Result<()>
{
    let output = bubblewrap.run_sandbox(true,"base", "/", "pacman", vec![
        "-Syy"
    ]).await?;

    if !output.status.success() {
        error!(
                "Error while updating repos, continuing without update ...\n---stdout---\n{:?}\n---stderr---\n{:?}",
                String::from_utf8(output.stdout),
                String::from_utf8(output.stderr)
            );
        bail!("Failed to update pacman repos with code {:?}", output.status.code());
    }

    Ok(())
}

pub async fn is_package_in_repo(bubblewrap: &Bubblewrap, package_name: &String) -> bool {
    let output = bubblewrap.run_sandbox(true,"base", "/", "pacman", vec![
        "-Ss",
        format!("^{}$", package_name).as_str(),
    ]).await;

    if output.is_err() {
        warn!("Could not check pacman for {}, {}", package_name, output.unwrap_err());
        return false;
    }
    output.unwrap().status.success()
}