use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::models::{Package, PackageStatus};

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
    pub package: Package,
    pub status: PackageStatus,
    pub last_built: Option<DateTime<Utc>>,
}