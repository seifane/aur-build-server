use std::convert::Infallible;
use std::path::{Component, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock};
use warp::multipart::FormData;
use warp::{reply};
use warp::http::StatusCode;
use common::http::payloads::PackageRebuildPayload;
use common::http::responses::{SuccessResponse, WorkerResponse};
use common::models::PackageStatus;
use crate::http::util::MultipartField::{Text};
use crate::http::util::{parse_multipart};
use crate::orchestrator::Orchestrator;
use crate::orchestrator::state::PackageBuildData;

pub async fn get_packages(orchestrator: Arc<RwLock<Orchestrator>>) -> Result<impl warp::Reply, Infallible> {
    Ok::<_, Infallible>(warp::reply::json(&orchestrator.read().await.state.get_packages().iter().map(|i| i.1.as_http_response()).collect::<Vec<_>>()))
}

pub async fn get_workers(orchestrator: Arc<RwLock<Orchestrator>>) -> Result<impl warp::Reply, Infallible> {
    Ok::<_, Infallible>(warp::reply::json(&orchestrator.read().await.workers.values().map(|it| it.to_http_response()).collect::<Vec<WorkerResponse>>()))
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
    if let Some(packages) = payload.packages {
        for package in packages.iter() {
            orchestrator.write().await.state.set_package_status(package, PackageStatus::PENDING);
        }
    } else {
        orchestrator.write().await.rebuild_all_packages();
    }

    Ok(reply::json(&SuccessResponse::from(true)))
}

pub async fn webhook_trigger_package(orchestrator: Arc<RwLock<Orchestrator>>, package_name: String) -> Result<impl warp::Reply, Infallible>
{
    let orchestrator_lock = orchestrator.write().await;
    let package = orchestrator_lock.state.get_package_by_name(&package_name);

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

    orchestrator.write().await.handle_package_build_response(
        package_name,
        PackageBuildData {
            files: fields.get("files[]"),
            log_files: fields.get("log_files[]"),

            errors: parsed_errors,
            time: Some(chrono::offset::Utc::now()),
            version: built_version,
        }
    ).await;
    Ok("")
}
