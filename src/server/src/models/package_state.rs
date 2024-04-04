use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use common::models::PackageStatus;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageState {
    pub status: PackageStatus,
    pub last_built: Option<DateTime<Utc>>,
    pub files: Vec<String>,
    pub last_built_version: Option<String>
}

impl PackageState {
    pub fn new() -> Self
    {
        PackageState {
            status: PackageStatus::PENDING,
            last_built: None,
            files: Vec::new(),
            last_built_version: None,
        }
    }
}