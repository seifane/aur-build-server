use crate::models::{PackageStatus, WorkerStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool
}

impl SuccessResponse {
    pub fn from(success: bool) -> SuccessResponse
    {
        SuccessResponse {
            success,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageResponse {
    pub id: i32,
    pub name: String,
    pub run_before: Option<String>,
    pub status: PackageStatus,
    pub last_built: Option<DateTime<Utc>>,
    pub files: Vec<String>,
    pub last_built_version: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackagePatchResponse {
    pub id: i32,
    pub package_id: i32,
    pub url: String,
    pub sha_512: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorkerResponse {
    pub id: usize,
    pub status: WorkerStatus,
    pub current_job: Option<String>,
    pub is_authenticated: bool
}