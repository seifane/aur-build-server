use log::info;
use serde::Serialize;
use chrono::DateTime;
use chrono::offset::Utc;
use common::http::responses::{PackageResponse};
use common::models::PackageStatus;
use crate::models::config::PackageConfig;

#[derive(Serialize, Debug, Clone)]
pub struct Package {
    pub name: String,
    pub run_before: Option<String>,
    pub status: PackageStatus,
    pub last_built: Option<DateTime<Utc>>,
    pub last_built_version: Option<String>
}

impl Package {
    pub fn from_package_config(package_config: &PackageConfig) -> Package {
        Package {
            name: package_config.name.clone(),
            run_before: package_config.run_before.clone(),
            status: PackageStatus::PENDING,
            last_built: None,
            last_built_version: None,
        }
    }

    pub fn set_status(&mut self, status: PackageStatus) {
        info!("Package {} status changed to {:?}", self.name, status);
        self.status = status;
    }

    pub fn to_response(&self) -> PackageResponse
    {
        PackageResponse {
            name: self.name.clone(),
            status: self.status.clone(),
            run_before: self.run_before.clone(),
            last_built: self.last_built.clone(),
            last_built_version: self.last_built_version.clone()
        }
    }
}