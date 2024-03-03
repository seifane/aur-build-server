use std::error::Error;
use log::{debug, error, info};
use tokio::process::Command;
use crate::models::config::Config;

pub struct Repo {
    pub repo_name: String,
    pub sign_key: Option<String>,
}

impl Repo {
    pub fn new(repo_name: String, sign_key: Option<String>) -> Self
    {
        Repo {
            repo_name,
            sign_key,
        }
    }

    pub fn from_config(config: &Config) -> Self
    {
        Self::new(config.repo_name.clone(), config.sign_key.clone())
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
        if let Some(sign_key) = self.sign_key.as_ref() {
            for file in package_files.iter() {
                let out = Command::new("gpg")
                    .arg("--default-key")
                    .arg(sign_key)
                    .arg("--yes")
                    .arg("--output")
                    .arg(format!("serve/{}.sig", file))
                    .arg("--detach-sig")
                    .arg(format!("serve/{}", file))
                    .output().await?;

                if !out.status.success() {
                    error!("GPG failed with exit code : {} : {:?} {:?}", out.status.code().unwrap_or(-1), String::from_utf8(out.stdout), String::from_utf8(out.stderr));
                } else {
                    debug!("GPG output for {} exit code : {:?} {:?} {:?}", file, out.status.code(), String::from_utf8(out.stdout), String::from_utf8(out.stderr))
                }
            }
        } else {
            info!("Skipping signature ...")
        }

        return self.build_repo(package_files).await;
    }

    async fn build_repo(&self, mut package_files: Vec<String>) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Building repo");

        if package_files.is_empty() {
            package_files = self.get_packages().await?;
        }

        let mut args = vec!["--remove"];
        if let Some(sign_key) = self.sign_key.as_ref() {
            args.push("--verify");
            args.push("--sign");
            args.push("--key");
            args.push(sign_key.as_str());
        }

        let repo_output_name = format!("{}.db.tar.gz", self.repo_name);
        args.push(repo_output_name.as_str());

        for package in package_files.iter() {
            args.push(package.as_str());
        }

        let out = Command::new("repo-add")
            .current_dir("serve/")
            .args(args)
            .output().await?;

        debug!("repo-add output exit code : {:?} {:?} {:?}", out.status.code() ,String::from_utf8(out.stdout), String::from_utf8(out.stderr));

        Ok(())
    }
}



