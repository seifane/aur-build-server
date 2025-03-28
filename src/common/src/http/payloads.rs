use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PackageRebuildPayload
{
    pub packages: Option<Vec<i32>>,
    pub force: Option<bool>
}

#[derive(Serialize, Deserialize)]
pub struct PostPackage {
    pub name: String,
    pub run_before: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PatchPackage {
    pub run_before: Option<String>,
}
