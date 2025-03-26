use serde::{Deserialize, Serialize};
use crate::models::PackageJob;
use crate::models::WorkerStatus;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "payload")]
pub enum WebsocketMessage {
    Authenticate {
      api_key: String,
    },
    JobSubmit {
        package: PackageJob,
    },
    WorkerStatusRequest {},
    WorkerStatusUpdate {
        status: WorkerStatus,
        job: Option<PackageJob>
    },
    UploadArtifactRequest {},
    UploadArtifactResponse {
        path: String
    },
}