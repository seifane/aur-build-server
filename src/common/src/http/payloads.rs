use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PackageRebuildPayload
{
    pub packages: Option<Vec<i32>>,
    pub force: Option<bool>
}