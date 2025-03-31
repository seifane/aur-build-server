use actix_multipart::form::MultipartForm;
use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::text::Text;
use actix_web::{web, Error, HttpRequest, HttpResponse, Scope};
use actix_web::web::{scope, Json};
use log::{debug, error, info};
use crate::http::base::{JsonResult, SuccessResponse};
use crate::http::HttpState;

pub fn register() -> Scope {
    scope("/api_workers")
        .route("/upload", web::post().to(upload))
        .route("/ws", web::get().to(websocket))
}

#[derive(Debug, MultipartForm)]
struct UploadForm {
    pub package_name: Text<String>,
    pub version: Text<String>,
    pub error: Option<Text<String>>,

    pub log_files: Vec<TempFile>,
    pub files: Vec<TempFile>,
}

async fn upload(
    MultipartForm(form): MultipartForm<UploadForm>,
    state: web::Data<HttpState>,
) -> JsonResult<SuccessResponse>
{
    debug!("Received upload from worker {:?}", form);

    let built_version = match form.version.as_str() {
        "" => None,
        _ => Some(form.version.to_string()),
    };

    let res = state.orchestrator.write().await
        .handle_package_build_output(
            form.package_name.to_string(),
            built_version,
            form.error.map(|o| o.to_string()),
            form.log_files,
            form.files,
        ).await;

    match res {
        Ok(_) => info!("Successfully handled build output for package {}", form.package_name.to_string()),
        Err(e) => error!("Failed to handle build output for package {}: '{}'", form.package_name.to_string(), e),
    }

    Ok(Json(SuccessResponse::from(true)))
}

async fn websocket(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<HttpState>,
) -> Result<HttpResponse, Error>
{
    let (res, session, stream) = actix_ws::handle(&req, stream)?;

    let stream = stream
        .aggregate_continuations();

    state.orchestrator.write().await.add_worker(session, stream).await;

    Ok(res)
}