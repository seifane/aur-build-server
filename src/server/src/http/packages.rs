use crate::http::base::{HttpError, JsonResult, ResponseResult, SuccessResponse};
use crate::http::HttpState;
use crate::persistence::package_store::PackageInsert;
use actix_web::http::StatusCode;
use actix_web::web::{scope, Json};
use actix_web::{web, HttpResponse, Scope};
use anyhow::anyhow;
use common::http::payloads::{PackageRebuildPayload, PatchPackage, PostPackage};
use common::http::responses::PackageResponse;
use std::path::Component;

pub fn register() -> Scope {
    scope("/packages")
        .route("", web::get().to(index))
        .route("", web::post().to(post))
        .route("/rebuild", web::post().to(rebuild))
        .route("/{id}", web::patch().to(patch))
        .route("/{id}", web::delete().to(delete))
        .route("/{id}/logs", web::get().to(action_logs))
}

async fn index(state: web::Data<HttpState>) -> JsonResult<Vec<PackageResponse>> {
    let mut packages = Vec::new();
    for package in state
        .orchestrator
        .write()
        .await
        .get_package_store()
        .get_packages()
        .await?
    {
        packages.push(package.into_package_response());
    }
    Ok(Json(packages))
}

async fn post(state: web::Data<HttpState>, body: Json<PostPackage>) -> JsonResult<PackageResponse> {
    let body = body.into_inner();

    let package = state.orchestrator.write().await
        .get_package_store()
        .create_package(PackageInsert {
            name: body.name,
            run_before: body.run_before,
        }).await?;
    Ok(Json(package.into_package_response()))
}

async fn rebuild(state: web::Data<HttpState>, body: Json<PackageRebuildPayload>) -> JsonResult<SuccessResponse> {
    let body = body.into_inner();

    state.orchestrator.write().await
        .get_package_store()
        .set_packages_pending(body.packages, body.force.unwrap_or(false))
        .await?;

    Ok(Json(SuccessResponse::from(true)))
}

async fn patch(state: web::Data<HttpState>, path: web::Path<i32>, body: Json<PatchPackage>) -> JsonResult<PackageResponse>
{
    let id = path.into_inner();
    let body = body.into_inner();

    let mut orchestrator = state.orchestrator.write().await;
    if let Some(mut package) = orchestrator.get_package_store().get_package(id).await? {
        package.run_before = body.run_before;
        orchestrator.get_package_store().update_package(&package).await?;
        return Ok(Json(package.into_package_response()));
    }

    Err(HttpError::not_found())
}

async fn delete(state: web::Data<HttpState>, id: web::Path<i32>) -> JsonResult<SuccessResponse> {
    let res = state
        .orchestrator
        .write()
        .await
        .get_package_store()
        .delete_package(id.into_inner())
        .await;
    Ok(Json(SuccessResponse::from(res.is_ok())))
}

async fn action_logs(
    state: web::Data<HttpState>,
    id: web::Path<i32>,
) -> ResponseResult {
    if let Some(package) = state
        .orchestrator
        .write()
        .await
        .get_package_store()
        .get_package(id.into_inner()).await?
    {
        let path = state
            .config
            .read()
            .await
            .build_logs_path
            .join(format!("{}.log", package.get_name()));
        if path.components().into_iter().any(|x| x == Component::ParentDir) {
            return Err(HttpError::new(anyhow!("Bad request"), StatusCode::BAD_REQUEST));
        }

        let content = tokio::fs::read_to_string(&path).await.unwrap_or_else(|e| {
            format!("Failed to read file: {}", e)
        });
        return Ok(HttpResponse::Ok().body(content));
    }

    Err(HttpError::not_found())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;
    use tokio::io::AsyncWriteExt;
    use common::models::PackageStatus;
    use crate::get_test_app;

    #[actix_web::test]
    async fn test_index_packages() {
        let (app, _) = get_test_app!();
        let req = test::TestRequest::get().uri("/api/packages").to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;

        let parsed: Vec<PackageResponse> = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.len(), 2);
    }

    #[actix_web::test]
    async fn test_post_packages() {
        let (app, state) = get_test_app!();
        let req = test::TestRequest::post()
            .uri("/api/packages")
            .set_json(PostPackage {
                name: "test-insert".to_string(),
                run_before: Some("testrun".to_string()),
            })
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;

        let parsed: PackageResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.name, "test-insert");
        assert_eq!(parsed.run_before, Some("testrun".to_string()));
        assert_eq!(parsed.status, PackageStatus::PENDING);

        let packages = state.orchestrator.write().await.get_package_store().get_packages().await.unwrap();
        assert_eq!(packages.len(), 3);
    }

    #[actix_web::test]
    async fn test_rebuild_packages() {
        let (app, state) = get_test_app!();

        let req = test::TestRequest::post()
            .uri("/api/packages/rebuild")
            .set_json(PackageRebuildPayload {
                packages: Some(vec![2]),
                force: Some(true),
            })
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let parsed: SuccessResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.success, true);

        let packages = state.orchestrator.write().await.get_package_store().get_package(2).await.unwrap().unwrap();
        assert_eq!(packages.get_status(), PackageStatus::PENDING);
        assert!(packages.last_built_version.is_none());
    }

    #[actix_web::test]
    async fn test_patch_packages() {
        let (app, state) = get_test_app!();

        let req = test::TestRequest::patch()
            .uri("/api/packages/1")
            .set_json(PatchPackage {
                run_before: Some("run_before_update".to_string()),
            })
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let parsed: PackageResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(parsed.run_before, Some("run_before_update".to_string()));
        let package = state.orchestrator.write().await.get_package_store().get_package(1).await.unwrap().unwrap();
        assert_eq!(package.run_before, Some("run_before_update".to_string()));
    }

    #[actix_web::test]
    async fn test_delete_packages() {
        let (app, state) = get_test_app!();

        let req = test::TestRequest::delete()
            .uri("/api/packages/1")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let parsed: SuccessResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(parsed.success, true);
        let packages = state.orchestrator.write().await.get_package_store().get_packages().await.unwrap();
        assert_eq!(packages.len(), 1);
    }

    #[actix_web::test]
    async fn test_action_logs_packages() {
        let (app, state) = get_test_app!();

        let base_path = state.config.read().await.build_logs_path.clone();
        tokio::fs::create_dir_all(&base_path).await.unwrap();
        let path = base_path.join("first.log");

        {
            let mut file = tokio::fs::File::create(path).await.unwrap();
            file.write_all("test file content".as_bytes()).await.unwrap();
            file.flush().await.unwrap();
        }

        let req = test::TestRequest::get()
            .uri("/api/packages/1/logs")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;

        assert_eq!(String::from_utf8(body.to_vec()).unwrap(), "test file content");
    }
}