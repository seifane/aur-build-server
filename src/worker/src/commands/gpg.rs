use std::path::PathBuf;
use log::{info, warn};
use crate::builder::bubblewrap::Bubblewrap;
use crate::commands::makepkg::get_src_info;

pub async fn attempt_recv_pgp_keys(bubblewrap: &Bubblewrap, data_path: &PathBuf, package_name: &String) {
    info!("Attempting to fetch PGP keys");

    match get_src_info(data_path, package_name).await {
        Ok(src_info) => {
            for key in src_info.base.valid_pgp_keys.iter() {
                info!("Trying to fetch {} public key", key);

                let res = bubblewrap.run_sandbox(true, "current", "/", "gpg", vec![
                    "--auto-key-locate",
                    "nodefault,wkd",
                    "--receive-keys",
                    key.as_str(),
                ], None, None).await;
                match res {
                    Ok(output) => {
                        if output.status.success() {
                            info!("Successfully received PGP key {}", key);
                        } else {
                            warn!("Failed to receive PGP key {}: status code: {:?}", key, output.status.code());
                        }
                    }
                    Err(e) => {
                        warn!("Failed to receive PGP key {}: {}", key, e);
                    }
                }
            }
        }
        Err(e) => {
            warn!("Failed to parse srcinfo to retreive PGP key: {}", e);
        }
    }
}