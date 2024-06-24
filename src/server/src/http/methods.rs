use std::convert::Infallible;
use std::path::{Component, PathBuf};
use std::sync::Arc;
use handlebars::Handlebars;
use serde::Serialize;
use tokio::fs::read_to_string;
use tokio::sync::{RwLock};
use warp::multipart::FormData;
use warp::{reply};
use warp::http::StatusCode;
use common::http::payloads::PackageRebuildPayload;
use common::http::responses::{SuccessResponse, WorkerResponse};
use crate::http::models::PackageBuildData;
use crate::http::multipart::MultipartField::{Text};
use crate::http::multipart::{parse_multipart};
use crate::orchestrator::Orchestrator;

pub async fn get_packages(orchestrator: Arc<RwLock<Orchestrator>>) -> Result<impl warp::Reply, Infallible> {
    Ok::<_, Infallible>(warp::reply::json(&orchestrator.read().await.repository.get_packages().iter().map(|i| i.1.as_http_response()).collect::<Vec<_>>()))
}

pub async fn get_workers(orchestrator: Arc<RwLock<Orchestrator>>) -> Result<impl warp::Reply, Infallible> {
    Ok::<_, Infallible>(warp::reply::json(&orchestrator.read().await.worker_manager.workers.values().map(|it| it.to_http_response()).collect::<Vec<WorkerResponse>>()))
}

pub async fn get_logs(package: String) -> Result<impl warp::Reply, Infallible> {
    let path = PathBuf::from(format!("logs/{}.log", package));
    // Prevent path traversal
    if path.components().into_iter().any(|x| x == Component::ParentDir) {
        return Ok(reply::with_status("".to_string(), StatusCode::INTERNAL_SERVER_ERROR));
    }

    let content = tokio::fs::read_to_string(path.to_str().unwrap()).await.unwrap_or("".to_string());
    Ok(reply::with_status(content, StatusCode::OK))
}

pub async fn rebuild_packages(orchestrator: Arc<RwLock<Orchestrator>>, payload: PackageRebuildPayload) -> Result<impl warp::Reply, Infallible>
{
    let force = payload.force.unwrap_or(false);

    if let Some(packages) = payload.packages {
        for package in packages.iter() {
            orchestrator.write().await.repository.queue_package_for_rebuild(package, force);
        }
    } else {
        orchestrator.write().await.repository.queue_all_packages_for_rebuild(force);
    }

    Ok(reply::json(&SuccessResponse::from(true)))
}

pub async fn webhook_trigger_package(orchestrator: Arc<RwLock<Orchestrator>>, package_name: String) -> Result<impl warp::Reply, Infallible>
{
    let orchestrator_lock = orchestrator.write().await;
    let package = orchestrator_lock.repository.get_package_by_name(&package_name);

    match package {
        None => {
            Ok(reply::json(&SuccessResponse::from(false)))
        }
        Some(package) => {
            orchestrator_lock.webhook_manager.trigger_webhook_package_updated(package.as_http_response()).await;
            Ok(reply::json(&SuccessResponse::from(true)))
        }
    }
}


pub async fn upload_package(orchestrator: Arc<RwLock<Orchestrator>>, form: FormData) -> Result<impl warp::Reply, Infallible>
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

    orchestrator.write().await.repository.handle_package_build_output(
        package_name,
        PackageBuildData {
            files: fields.get("files[]"),
            log_files: fields.get("log_files[]"),

            errors: parsed_errors,
            version: built_version,
        }
    ).await;
    Ok("")
}

#[derive(Serialize)]
struct IndexRepoData {
    pub name: String,
    pub files: Vec<String>
}

pub async fn index_repo(_: Arc<RwLock<Orchestrator>>) -> Result<impl warp::Reply, Infallible>
{
    let mut reg = Handlebars::new();

    // let directory = orchestrator.read().await.con

    let template_content = read_to_string("./src/pages/repo_index.html").await.unwrap();
    let rendred = reg.render_template(template_content.as_str(), &IndexRepoData{
        name: "test".to_string(),
        files: Vec::new()
    }).unwrap();

    Ok(warp::reply::html(rendred))
}
