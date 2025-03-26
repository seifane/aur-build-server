use std::convert::Infallible;
use std::os::unix::fs::MetadataExt;
use std::path::Component;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use handlebars::Handlebars;
use log::{error, info};
use serde::Serialize;
use tokio::fs::read_dir;
use tokio::sync::RwLock;
use warp::{reply, Reply};
use warp::http::StatusCode;
use warp::multipart::FormData;

use common::http::payloads::PackageRebuildPayload;
use common::http::responses::{SuccessResponse, WorkerResponse};

use crate::http::models::PackageBuildData;
use crate::http::multipart::MultipartField::Text;
use crate::http::multipart::parse_multipart;
use crate::models::config::Config;
use crate::orchestrator::Orchestrator;

pub async fn get_packages(orchestrator: Arc<RwLock<Orchestrator>>) -> Result<impl Reply, Infallible> {
    Ok::<_, Infallible>(
        reply::json(
            &orchestrator
                .write().await
                .get_package_store()
                .get_packages().await.unwrap()
                .into_iter()
                .map(|i| i.into_package_response())
                .collect::<Vec<_>>()))
}

pub async fn get_workers(orchestrator: Arc<RwLock<Orchestrator>>) -> Result<impl Reply, Infallible> {
    Ok::<_, Infallible>(reply::json(&orchestrator.read().await.worker_manager.workers.values().map(|it| it.to_http_response()).collect::<Vec<WorkerResponse>>()))
}

pub async fn remove_worker(orchestrator: Arc<RwLock<Orchestrator>>, id: usize) -> Result<impl Reply, Infallible> {
    let worker = orchestrator.write().await.worker_manager.remove(id);
    Ok(reply::json(&SuccessResponse::from(worker.is_some())))
}

pub async fn get_logs(config: Arc<RwLock<Config>>, package: String) -> Result<impl Reply, Infallible> {
    let path = config.read().await.build_logs_path.join(format!("{}.log", package));
    // Prevent path traversal
    if path.components().into_iter().any(|x| x == Component::ParentDir) {
        return Ok(reply::with_status("".to_string(), StatusCode::INTERNAL_SERVER_ERROR));
    }

    let content = tokio::fs::read_to_string(path.to_str().unwrap()).await.unwrap_or_else(|e| {
        format!("Failed to read file: {}", e)
    });

    Ok(reply::with_status(content, StatusCode::OK))
}

pub async fn rebuild_packages(orchestrator: Arc<RwLock<Orchestrator>>, payload: PackageRebuildPayload) -> Result<impl Reply, Infallible>
{
    let force = payload.force.unwrap_or(false);

    let packages = if let Some(packages) = payload.packages {
        packages
    } else {
        orchestrator.write().await.get_package_store().get_packages().await.unwrap()
            .iter()
            .map(|i| i.id)
            .collect()
    };
    orchestrator.write().await.get_package_store().set_packages_pending(packages, force).await.unwrap();

    Ok(reply::json(&SuccessResponse::from(true)))
}

pub async fn webhook_trigger_package(orchestrator: Arc<RwLock<Orchestrator>>, package_name: String) -> Result<impl Reply, Infallible>
{
    let mut orchestrator_lock = orchestrator.write().await;
    let package = orchestrator_lock.get_package_store().get_package_by_name(&package_name).await;

    if let Ok(maybe_package) = package {
        return match maybe_package {
            None => {
                Ok(reply::json(&SuccessResponse::from(false)))
            }
            Some(package) => {
                orchestrator_lock.webhook_manager.trigger_webhook_package_updated(package.into_package_response()).await;
                Ok(reply::json(&SuccessResponse::from(true)))
            }
        };
    }

    Ok(reply::json(&SuccessResponse::from(false)))
}


pub async fn upload_package(orchestrator: Arc<RwLock<Orchestrator>>, form: FormData) -> Result<impl Reply, Infallible>
{
    let fields = parse_multipart(form).await?;

    let package_name = if let Some(package_name) = fields.get("package_name") {
        if let Text(package_name) = package_name.first().unwrap() {
            Some(package_name)
        } else {
            None
        }
    } else {
        None
    }.unwrap();

    let built_version = if let Some(version) = fields.get("version") {
        if let Text(version) = version.first().unwrap() {
            match version.as_str() {
                "" => None,
                s => Some(s.to_string())
            }
        } else {
            None
        }
    } else {
        None
    };

    let parsed_errors = match fields.get("error") {
        None => Vec::new(),
        Some(errors) => {
            let mut parsed = Vec::new();
            for error in errors.iter() {
                if let Text(error) = error {
                    parsed.push(error.to_string());
                }
            }
            parsed
        }
    };

    let res = orchestrator.write().await.handle_package_build_output(
        package_name,
        PackageBuildData {
            files: fields.get("files[]"),
            log_files: fields.get("log_files[]"),

            errors: parsed_errors,
            version: built_version,
        }
    ).await;

    match res {
        Ok(_) => info!("Successfully handled build output for package {}", package_name),
        Err(e) => error!("Failed to handle build output for package {}: '{}'", package_name, e),
    }
    Ok("")
}

#[derive(Serialize)]
struct IndexRepoData {
    pub name: String,
    pub files: Vec<File>
}

#[derive(Serialize)]
struct File {
    pub name: String,
    pub date_modified_iso: String,
    pub size: u64,
}

pub async fn index_repo(orchestrator: Arc<RwLock<Orchestrator>>) -> Result<impl Reply, Infallible>
{
    let template_content = include_str!("./pages/repo_index.html");
    let path = orchestrator.read().await.config.read().await.serve_path.clone();
    let mut files = Vec::new();

    if let Ok(mut dir) = read_dir(path).await {
        while let Some(entry) = dir.next_entry().await.unwrap() {
            let metadata = entry.metadata().await.unwrap();
            let dt: DateTime<Utc> = metadata.modified().unwrap().clone().into();
            files.push(File {
                name: entry.file_name().into_string().unwrap(),
                date_modified_iso: format!("{}", dt.format("%+")),
                size: metadata.size()
            });
        }
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));

    let reg = Handlebars::new();
    if let Ok(rendered) = reg.render_template(template_content, &IndexRepoData {
        name: orchestrator.read().await.config.read().await.repo_name.clone(),
        files
    }) {
        return Ok(reply::html(rendered).into_response())
    }


    let mut response = reply::html("Server Error").into_response();
    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    Ok(response)
}
