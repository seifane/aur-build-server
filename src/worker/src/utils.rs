use std::error::Error;
use std::path::Path;
use log::info;
use reqwest::multipart::{Form, Part};
use tokio::fs::read_dir;

pub fn sanitize_dependency(dep: &str) -> String {
    let mut char_index = 0;
    for c in vec![">", "<", "="] {
        let found = dep.find(c).unwrap_or(0);
        if char_index == 0 || found < char_index {
            char_index = found;
        }
    }
    if char_index > 0 {
        return dep[..char_index].to_string();
    }
    dep.to_string()
}

pub async fn add_package_files_to_form_data(package_name: &String, mut form: Form) -> Result<Form, Box<dyn Error + Send + Sync>>
{
    let repo_data_str = format!("data/{}", package_name).to_string();
    let repo_data_path = Path::new(repo_data_str.as_str());
    let mut dir = read_dir(repo_data_path).await?;

    while let Some(file) = dir.next_entry().await? {
        if file.file_name().to_str().unwrap().contains(".pkg.tar.zst") {
            let content = tokio::fs::read(file.path()).await?;
            form = form.part(
                "files[]",
                Part::bytes(content)
                    .file_name(file.file_name().into_string().unwrap())
            );
            info!("Uploading package file {}", file.file_name().to_str().unwrap_or(""))
        }
    }

    Ok(form)
}