use serde::{Serialize};
use crate::utils::package_data::Package;

#[derive(Serialize)]
pub struct BasicResultResponse {
    pub ok: bool,
}

#[derive(Serialize)]
pub struct BasicErrorResponse {
    pub ok: bool,
    pub error: String,
}

#[derive(Serialize)]
pub struct PackagesResponse {
    pub is_running: bool,
    pub commit_queued: bool,
    pub packages: Vec<Package>,
}