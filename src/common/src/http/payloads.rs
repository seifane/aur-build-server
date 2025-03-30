use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PackageRebuildPayload
{
    pub packages: Option<Vec<i32>>,
    pub force: Option<bool>
}

#[derive(Serialize, Deserialize)]
pub struct CreatePackagePayload {
    pub name: String,
    pub run_before: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdatePackagePayload {
    pub run_before: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CreatePackagePatchPayload {
    pub url: String,
    pub sha_512: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdatePackagePatchPayload {
    pub url: String,
    pub sha_512: Option<String>,
}
