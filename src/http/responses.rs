use serde::{Serialize};
use crate::package_manager::Package;

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
pub struct PackagesResponse<'a> {
    pub is_running: bool,
    pub commit_queued: bool,
    pub packages: &'a Vec<Package>,
}