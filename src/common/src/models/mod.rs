use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PackagePatch {
    pub url: String,
    pub sha512: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Package {
    pub name: String,
    pub run_before: Option<String>,
    pub patches: Vec<PackagePatch>,
    pub last_built_version: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub enum PackageStatus {
    PENDING,
    BUILDING,
    BUILT,
    FAILED,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum WorkerStatus {
    UNKNOWN,
    STANDBY,
    DISPATCHED,
    UPDATING,
    WORKING,
    UPLOADING,
    CLEANING,
}