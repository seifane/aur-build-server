use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::models::PackageStatus;

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
    pub name: String,
    pub status: PackageStatus,
    pub run_before: Option<String>,
    pub last_built: Option<DateTime<Utc>>,
    pub last_built_version: Option<String>
}