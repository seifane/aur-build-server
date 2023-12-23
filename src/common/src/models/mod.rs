use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
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