use crate::http::base::{JsonResult, SuccessResponse};
use crate::http::HttpState;
use actix_web::web::{scope, Json};
use actix_web::{web, Scope};

pub fn register() -> Scope {
    scope("/webhooks")
        .route("trigger", web::post().to(trigger))
}

async fn trigger(state: web::Data<HttpState>) -> JsonResult<SuccessResponse> {
    state.orchestrator.read().await.send_test_webhook().await;
    Ok(Json(SuccessResponse::from(true)))
}