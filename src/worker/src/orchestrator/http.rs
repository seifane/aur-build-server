use anyhow::{Context, Result};
use log::{error, info};
use reqwest::multipart::Form;
use tokio::fs::{read_dir};
use crate::models::config::Config;
use crate::models::package_build_result::PackageBuildResult;
use crate::utils::get_package_dir_entries;

pub struct HttpClient {
    config: Config,
}

impl HttpClient {
    pub fn from_config(config: &Config) -> HttpClient
    {
        HttpClient {
            config: config.clone(),
        }
    }

    async fn add_logs_to_form(&self, mut form: Form) -> Result<Form> {
        let mut dir = read_dir(&self.config.build_logs_path).await?;
        while let Some(entry) = dir.next_entry().await? {
            let file_name = entry.file_name().into_string().unwrap();

            info!("Uploading log file {}", file_name);
            form = form.file("log_files", entry.path()).await?;
        }

        Ok(form)
    }

    async fn add_package_files_to_form_data(&self, mut form: Form) -> Result<Form>
    {
        let packages = get_package_dir_entries(&self.config.data_path.join("_built")).await?;

        for entry in packages.iter() {
            let file_name = entry.file_name().into_string().unwrap();

            info!("Uploading package file {}", file_name);
            form = form.file("files", entry.path()).await?;
        }

        Ok(form)
    }

    async fn build_form(&self, package_name: &String, build_result: Result<PackageBuildResult>) -> Result<Form> {
        let mut form = Form::new()
            .text("package_name", package_name.clone());

        form = match build_result {
            Ok(result) => {
                if result.built {
                    form = self.add_package_files_to_form_data(form).await.with_context(|| "Failed to add packages files to form")?;
                }
                form.text("version", result.version)
            }
            Err(e) => {
                form.text("error", format!("{:#}", e))
            }
        };

        Ok(
            self.add_logs_to_form(form).await.with_context(|| "Failed to add logs to form")?
        )
    }


    pub async fn upload_packages(
        &self,
        package_name: &String,
        build_result: Result<PackageBuildResult>
    ) -> Result<()>
    {
        let form = self.build_form(package_name, build_result).await?;

        let res = reqwest::Client::new()
            .post(format!("{}/api_workers/upload", self.config.base_url))
            .header("Authorization", &self.config.api_key)
            .multipart(form)
            .send().await;

        match res {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Upload response {}", response.status())
                } else {
                    error!("Upload error {}: '{}'", response.status(), response.text().await?);
                }
            }
            Err(err) => {error!("Error uploading {:?}", err)}
        }

        Ok(())
    }
}

