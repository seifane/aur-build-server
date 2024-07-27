use anyhow::{Context, Result};
use std::path::PathBuf;

use log::{debug, error, info};
use tokio::fs::{remove_file, try_exists};
use tokio::process::Command;

use crate::models::config::Config;

pub struct RepositoryManager {
    pub repo_name: String,
    pub sign_key: Option<String>,
    pub path: PathBuf,
}

impl RepositoryManager {
    pub async fn new(repo_name: String, sign_key: Option<String>, path: PathBuf) -> Result<Self>
    {
        let instance = RepositoryManager {
            repo_name,
            sign_key,
            path,
        };

        if !instance.path.exists() {
            tokio::fs::create_dir_all(&instance.path).await?;
        }

        Ok(instance)
    }

    pub async fn from_config(config: &Config) -> Result<Self>
    {
        Self::new(config.repo_name.clone(), config.sign_key.clone(), config.serve_path.clone()).await
    }

    #[allow(dead_code)]
    pub async fn get_package_files(&self) -> Result<Vec<String>>
    {
        let mut dir = tokio::fs::read_dir(&self.path).await
            .with_context(|| format!("Failed to read directory {:?}", self.path))?;

        let mut packages = Vec::new();

        while let Some(entry) = dir.next_entry().await? {
            if entry.file_name().to_str().unwrap().ends_with(".pkg.tar.zst") {
                packages.push(entry.file_name().to_str().unwrap().to_string());
            }
        }

        Ok(packages)
    }

    pub async fn add_packages_to_repo(&self, package_files: Vec<String>) -> Result<()> {
        if let Some(sign_key) = self.sign_key.as_ref() {
            for file in package_files.iter() {
                let out = Command::new("gpg")
                    .arg("--default-key")
                    .arg(sign_key)
                    .arg("--yes")
                    .arg("--output")
                    .arg(self.path.join(format!("{}.sig", file)).to_str().unwrap())
                    .arg("--detach-sig")
                    .arg(self.path.join(file))
                    .output().await?;

                if !out.status.success() {
                    error!("GPG failed with exit code : {} : {:?} {:?}", out.status.code().unwrap_or(-1), String::from_utf8(out.stdout), String::from_utf8(out.stderr));
                } else {
                    debug!("GPG output for {} exit code : {:?} {:?} {:?}", file, out.status.code(), String::from_utf8(out.stdout), String::from_utf8(out.stderr))
                }
            }
        } else {
            info!("Skipping signature ...");
            for file in package_files.iter() {
                let sig_path = self.path.join(format!("{}.sig", file));
                if let Ok(exists) = try_exists(&sig_path).await {
                    if exists {
                        match remove_file(&sig_path).await {
                            Ok(_) => info!("Removed old signature {:?}", sig_path),
                            Err(e) => error!("Failed to remove old signature {:?} : {}", sig_path, e)
                        }
                    }
                }
            }
        }

        return self.repo_add_cmd(package_files).await;
    }

    async fn repo_add_cmd(&self, package_files: Vec<String>) -> Result<()> {
        debug!("Building repository");

        if package_files.is_empty() {
            return Ok(());
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
            .current_dir(&self.path)
            .args(&args)
            .output()
            .await
            .with_context(|| format!("Failed to run repo-add with args {:?}", args))?;

        debug!("repository-add output exit code : {:?} {:?} {:?}", out.status.code() ,String::from_utf8(out.stdout), String::from_utf8(out.stderr));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use serial_test::serial;

    use tokio::fs::{remove_dir_all, try_exists};

    use crate::repository::manager::RepositoryManager;

    async fn setup() -> RepositoryManager {
        let _ = remove_dir_all("/tmp/aur-build-server-test").await;

        RepositoryManager::new("test".to_string(), None, PathBuf::from("/tmp/aur-build-server-test/repo"))
            .await.unwrap()
    }

    #[tokio::test]
    #[serial]
    async fn can_add_package_to_repository_no_sign() {
        let manager = setup().await;

        tokio::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("aur-build-cli-0.10.0-1-any.pkg.tar.zst"),
            "/tmp/aur-build-server-test/repo/aur-build-cli-0.10.0-1-any.pkg.tar.zst"
        ).await.unwrap();

        let files = manager.get_package_files().await.unwrap();
        assert_eq!(1, files.len());

        manager.add_packages_to_repo(files).await.unwrap();

        assert_eq!(true, try_exists("/tmp/aur-build-server-test/repo/test.db").await.unwrap())
    }
}

