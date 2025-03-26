use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PackagePatch {
    pub url: String,
    pub sha512: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PackageDefinition {
    pub package_id: i32,
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
#[repr(u8)]
pub enum PackageStatus {
    UNKNOWN = 0,
    PENDING = 1,
    BUILDING = 2,
    BUILT = 3,
    FAILED = 4,
}

impl PackageStatus {
    pub fn from_u8(value: u8) -> PackageStatus {
        match value {
            1 => PackageStatus::PENDING,
            2 => PackageStatus::BUILDING,
            3 => PackageStatus::BUILT,
            4 => PackageStatus::FAILED,
            _ => PackageStatus::UNKNOWN,
        }
    }
}

impl Into<i16> for PackageStatus {
    fn into(self) -> i16 {
        self as u8 as i16
    }
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