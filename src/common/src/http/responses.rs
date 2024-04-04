use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::models::{PackageDefinition, PackageStatus, WorkerStatus};

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
    pub package: PackageDefinition,
    pub status: PackageStatus,
    pub last_built: Option<DateTime<Utc>>,
    pub last_built_version: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorkerResponse {
    pub id: usize,
    pub status: WorkerStatus,
    pub current_job: Option<String>,
    pub is_authenticated: bool
}