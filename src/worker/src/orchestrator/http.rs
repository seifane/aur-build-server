use std::error::Error;
use std::path::Path;
use log::{error, info};
use reqwest::{multipart};
use reqwest::multipart::Part;
use tokio::fs::{read_dir};
use crate::errors::PackageBuildError;
use crate::models::PackageBuild;
use crate::utils::add_package_files_to_form_data;

pub struct HttpClient {
    base_url: String,
    api_key: String
}

impl HttpClient {
    pub fn new(base_url: String, api_key: String) -> HttpClient
    {
        HttpClient {
            base_url,
            api_key,
        }
    }

    // TODO: Stream the payload instead of loading it
    pub async fn upload_packages(
        &self,
        package_name: &String,
        build_result: Result<PackageBuild, PackageBuildError>
    ) -> Result<(), Box<dyn Error + Sync + Send>>
    {
        let mut form = multipart::Form::new()
            .text("package_name", package_name.clone());

        form = match build_result {
            Ok(result) => {
                if result.built {
                    let mut packages = result.additional_packages.clone();
                    packages.push(package_name.clone());
                    for package in packages.iter() {
                        form = add_package_files_to_form_data(package, form).await?;
                    }
                }
                form.text("version", result.version)
            }
            Err(e) => {
                form.text("error", e.message)
                    .text("version", "".to_string())
            }
        };

        let logs_path = Path::new("worker_logs/");
        let mut dir = read_dir(logs_path).await?;
        while let Some(file) = dir.next_entry().await? {
            let content = tokio::fs::read(file.path()).await?;
            form = form.part(
                "log_files[]",
                Part::bytes(content)
                    .file_name(file.file_name().into_string().unwrap())
            );
            info!("Uploading log file {}", file.file_name().to_str().unwrap_or(""))
        }

        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/api/worker/upload", self.base_url))
            .header("Authorization", self.api_key.clone())
            .multipart(form)
            .send().await;
        match res {
            Ok(response) => {info!("Upload response {}", response.status())}
            Err(err) => {error!("Error uploading {:?}", err)}
        }

        Ok(())
    }
}

