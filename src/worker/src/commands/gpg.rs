use std::path::PathBuf;
use log::{info, warn};
use crate::builder::bubblewrap::Bubblewrap;
use crate::commands::makepkg::get_src_info;

pub async fn attempt_recv_gpg_keys(bubblewrap: &Bubblewrap, data_path: &PathBuf, package_name: &String) {
    match get_src_info(data_path, package_name).await {
        Ok(src_info) => {
            for key in src_info.base.valid_pgp_keys.iter() {
                let res = bubblewrap.run_sandbox(true, "current", "/", "gpg", vec![
                    "--auto-key-locate",
                    "nodefault,wkd",
                    "--receive-keys",
                    key.as_str(),
                ], None, None).await;
                match res {
                    Ok(output) => {
                        if output.status.success() {
                            info!("Successfully received gpg key {}", key);
                        } else {
                            warn!("Failed to receive gpg key {}: status code: {:?}", key, output.status.code());
                        }
                    }
                    Err(e) => {
                        warn!("Failed to receive gpg key {}: {}", key, e);
                    }
                }
            }
        }
        Err(e) => {
            warn!("Failed to parse srcinfo to retreive gpg key: {}", e);
        }
    }
}