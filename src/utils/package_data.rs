use std::time::SystemTime;
use serde::Serialize;

use crate::utils::package_data::PackageStatus::Queued;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PackageStatus {
    Queued,
    QueuedForce,
    Building,
    Built,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct Package {
    pub name: String,
    pub run_before: Option<String>,
    pub status: PackageStatus,
    pub last_build_commit: Option<String>,
    pub time: SystemTime,
}

impl PartialEq for Package {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Package {
    pub fn from_package_name(package_name: &String) -> Package {
        Package {
            name: package_name.clone(),
            run_before: None,
            status: Queued,
            last_build_commit: None,
            time: SystemTime::now()
        }
    }
}
