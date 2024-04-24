use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PackageRebuildPayload
{
    pub packages: Option<Vec<String>>,
    pub force: Option<bool>
}