use actix_web::{web, Scope};
use actix_web::web::{scope, Json};
use common::http::responses::WorkerResponse;
use crate::http::base::{JsonResult, SuccessResponse};
use crate::http::HttpState;

pub fn register() -> Scope {
    scope("/workers")
        .route("", web::get().to(index))
        .route("/{id}", web::delete().to(delete))
}

async fn index(state: web::Data<HttpState>) -> JsonResult<Vec<WorkerResponse>>
{
    let mut response = Vec::new();
    for worker in state.orchestrator.read().await.get_worker_manager().get_workers() {
        response.push(worker.to_http_response().await);
    }
    Ok(Json(response))
}

async fn delete(state: web::Data<HttpState>, id: web::Path<usize>) -> JsonResult<SuccessResponse>
{
    state.orchestrator.write().await.remove_worker(id.into_inner()).await;
    Ok(Json(SuccessResponse::from(true)))
}