use serde::{Deserialize, Serialize};
use crate::models::WorkerStatus;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "payload")]
pub enum WebsocketMessage {
    Authenticate {
      api_key: String,
    },
    JobSubmit {
        package: String,
        run_before: Option<String>,
        last_built_version: Option<String>
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