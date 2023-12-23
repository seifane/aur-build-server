use std::error::Error;
use log::{debug};
use tokio::process::Command;
use crate::models::config::Config;

pub struct Repo {
    pub repo_name: String,
    pub sign: bool,
}

impl Repo {
    pub fn new(repo_name: String, sign: bool) -> Self
    {
        Repo {
            repo_name,
            sign,
        }
    }

    pub fn from_config(config: &Config) -> Self
    {
        Self::new(config.repo_name.clone(), config.get_sign())
    }

    pub async fn get_packages(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>>
    {
        let mut dir = tokio::fs::read_dir("serve/").await?;

        let mut packages = Vec::new();

        while let Some(entry) = dir.next_entry().await? {
            if entry.file_name().to_str().unwrap().ends_with(".pkg.tar.zst") {
                packages.push(entry.file_name().to_str().unwrap().to_string());
            }
        }

        Ok(packages)
    }

    pub async fn add_packages_to_repo(&self, package_files: Vec<String>) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.sign {
            for file in package_files.iter() {
                let out = Command::new("gpg")
                    .arg("--yes")
                    .arg("--output")
                    .arg(format!("serve/{}.sig", file))
                    .arg("--detach-sig")
                    .arg(format!("serve/{}", file))
                    .output().await?;
                debug!("GPG output for {} exit code : {:?} {:?} {:?}", file, out.status.code(), String::from_utf8(out.stdout), String::from_utf8(out.stderr))
            }
        } else {
            debug!("Skipping signature ...")
        }

        return self.build_repo(package_files).await;
    }

    async fn build_repo(&self, package_files: Vec<String>) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Building repo");

        let mut args = vec!["--remove".to_string()];
        if self.sign {
            args.push("--verify".to_string());
            args.push("--sign".to_string());
        }
        args.push(format!("{}.db.tar.gz", self.repo_name));

        if package_files.is_empty() {
            let packages = self.get_packages().await?;
            for package in packages {
                args.push(package);
            }
        } else {
            for package in package_files {
                args.push(package);
            }
        };

        let out = Command::new("repo-add")
            .current_dir("serve/")
            .args(args)
            .output().await?;

        debug!("repo-add output exit code : {:?} {:?} {:?}", out.status.code() ,String::from_utf8(out.stdout), String::from_utf8(out.stderr));

        Ok(())
    }
}



