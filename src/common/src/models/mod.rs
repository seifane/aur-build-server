use std::fmt;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PackagePatch {
    pub url: String,
    pub sha512: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PackageDefinition {
    pub name: String,
    pub run_before: Option<String>,
    pub patches: Option<Vec<PackagePatch>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PackageJob {
    pub definition: PackageDefinition,
    pub last_built_version: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub enum PackageStatus {
    PENDING,
    BUILDING,
    BUILT,
    FAILED,
}

impl fmt::Display for PackageStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum WorkerStatus {
    UNKNOWN,
    STANDBY,
    DISPATCHED,
    INIT,
    UPDATING,
    WORKING,
    UPLOADING,
    CLEANING,
}

impl fmt::Display for WorkerStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}