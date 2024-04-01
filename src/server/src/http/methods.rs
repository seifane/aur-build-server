use std::convert::Infallible;
use std::path::{Component, PathBuf};
use std::sync::Arc;
use log::{debug, error, info};
use tokio::sync::{Mutex, RwLock};
use warp::multipart::FormData;
use warp::{reply};
use warp::http::StatusCode;
use common::http::payloads::PackageRebuildPayload;
use common::http::responses::{SuccessResponse};
use common::models::PackageStatus;
use crate::http::util::MultipartField::{File, Text};
use crate::http::util::parse_multipart;
use crate::models::worker::Worker;
use crate::orchestrator::Orchestrator;
use crate::orchestrator::state::PackageBuildData;
use crate::utils::repo::{Repo};

pub async fn get_packages(orchestrator: Arc<RwLock<Orchestrator>>) -> Result<impl warp::Reply, Infallible> {
    Ok::<_, Infallible>(warp::reply::json(&orchestrator.read().await.state.get_packages().iter().map(|i| i.to_response()).collect::<Vec<_>>()))
}

pub async fn get_workers(orchestrator: Arc<RwLock<Orchestrator>>) -> Result<impl warp::Reply, Infallible> {
    Ok::<_, Infallible>(warp::reply::json(&orchestrator.read().await.workers.values().collect::<Vec<&Worker>>()))
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

pub async fn upload_package(orchestrator: Arc<RwLock<Orchestrator>>, form: FormData, repo: Arc<Mutex<Repo>>) -> Result<impl warp::Reply, Infallible>
{
    let fields = parse_multipart(form).await?;

    let mut package_files = Vec::new();

    let package_name = if let Some(package_name) = fields.get("package_name") {
        if let Text(package_name) = package_name.first().unwrap() {
            Some(package_name)
        } else {
            None
        }
    } else {
        None
    };

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

    if let Some(files) = fields.get("files[]") {
        for file in files.iter() {
            if let File(filename, content) = file {
                debug!("Copying {} ...", filename);
                tokio::fs::write(format!("serve/{}", filename), content).await.unwrap();
                package_files.push(filename.clone());
            }
        }
    }

    if let Some(files) = fields.get("log_files[]") {
        for file in files.iter() {
            if let File(filename, content) = file {
                tokio::fs::write(format!("logs/{}", filename), content).await.unwrap();
            }
        }
    }

    info!("Received packages {:?}", package_files);

    if !package_files.is_empty() {
        let res = repo.lock().await.add_packages_to_repo(package_files.clone()).await;
        if let Err(e) = &res {
            error!("Add to repo failed {}", e.to_string());

            orchestrator.write().await.state.set_package_status(package_name.unwrap(), PackageStatus::FAILED);
            return Ok("");
        }
    }

    if let Some(error) = fields.get("error") {
        error!("Error while building {} = {:?}", package_name.unwrap(), error.first().unwrap());

        orchestrator.write().await.state.set_package_status(package_name.unwrap(), PackageStatus::FAILED);
    } else {
        orchestrator.write().await.state.set_package_status(package_name.unwrap(), PackageStatus::BUILT);
    }

    orchestrator.write().await.state.set_package_build_data(
        package_name.unwrap(),
        PackageBuildData {
            time: Some(chrono::offset::Utc::now()),
            version: built_version,
            package_files
        }
    );

    Ok("")
}
