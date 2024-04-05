use serde::Serialize;
use common::http::responses::PackageResponse;

#[derive(Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum WebhookPayload {
    PackageUpdated(PackageResponse),
}