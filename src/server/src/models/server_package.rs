use log::info;
use serde::{Deserialize, Serialize};
use chrono::DateTime;
use chrono::offset::Utc;
use common::http::responses::{PackageResponse};
use common::models::{Package, PackageStatus};
use crate::models::config::PackageConfig;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerPackage {
    pub package: Package,
    pub status: PackageStatus,
    pub last_built: Option<DateTime<Utc>>,
    pub files: Vec<String>,
}

impl ServerPackage {
    pub fn from_package_config(package_config: &PackageConfig) -> ServerPackage {
        ServerPackage {
            package: package_config.to_package(),
            status: PackageStatus::PENDING,
            last_built: None,
            files: Vec::new(),
        }
    }

    pub fn set_status(&mut self, status: PackageStatus) {
        info!("Package {} status changed to {:?}", self.package.name, status);
        self.status = status;
    }

    pub fn get_package_name(&self) -> &String {
        &self.package.name
    }

    pub fn to_response(&self) -> PackageResponse
    {
        PackageResponse {
            package: self.package.clone(),
            status: self.status.clone(),
            last_built: self.last_built.clone(),
        }
    }

    pub fn restore_state(&mut self, from: &ServerPackage)
    {
        self.status = from.status;
        self.files = from.files.clone();
        self.last_built = from.last_built;
        self.package.last_built_version = from.package.last_built_version.clone();
    }
}