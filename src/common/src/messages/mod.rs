use serde::{Deserialize, Serialize};
use crate::models::Package;
use crate::models::WorkerStatus;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "payload")]
pub enum WebsocketMessage {
    Authenticate {
      api_key: String,
    },
    JobSubmit {
        package: Package,
    },
    WorkerStatusRequest {},
    WorkerStatusUpdate {
        status: WorkerStatus,
        package: Option<String>
    },
    UploadArtifactRequest {},
    UploadArtifactResponse {
        path: String
    },
}