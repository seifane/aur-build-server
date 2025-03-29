use crate::http::base::JsonResult;
use crate::http::HttpState;
use crate::persistence::package_store::PackagePatchInsert;
use actix_web::web::{scope, Json};
use actix_web::{web, Scope};
use common::http::payloads::CreatePackagePatchPayload;
use common::http::responses::{PackagePatchResponse, SuccessResponse};

pub fn register() -> Scope
{
    scope("/packages/{package_id}/patches")
        .route("", web::get().to(index))
        .route("", web::post().to(post))
        .route("/{id}", web::delete().to(delete))
}

async fn index(state: web::Data<HttpState>, path: web::Path<i32>) -> JsonResult<Vec<PackagePatchResponse>> {
    let patches =
        state.orchestrator.write().await.get_package_store()
            .get_patches_for_package(path.into_inner())
            .await?;

    Ok(Json(patches.into_iter().map(Into::into).collect()))
}

async fn post(
    state: web::Data<HttpState>,
    path: web::Path<i32>,
    body: Json<CreatePackagePatchPayload>
) -> JsonResult<PackagePatchResponse> {
    let body = body.into_inner();
    let patch = state.orchestrator.write().await
        .get_package_store().create_patch(PackagePatchInsert {
            package_id: path.into_inner(),
            url: body.url,
            sha_512: body.sha_512,
        }).await?;

    Ok(Json(patch.into()))
}

async fn delete(
    state: web::Data<HttpState>,
    path: web::Path<(i32, i32)>,
) -> JsonResult<SuccessResponse> {
    let (_, patch_id) = path.into_inner();

    state.orchestrator
        .write()
        .await
        .get_package_store()
        .delete_patch(patch_id)
        .await?;

    Ok(Json(SuccessResponse::from(true)))
}

#[cfg(test)]
mod tests {
    use actix_web::test;
    use common::http::payloads::CreatePackagePatchPayload;
    use common::http::responses::{PackagePatchResponse, SuccessResponse};
    use crate::get_test_app;

    #[actix_web::test]
    async fn test_index_patches() {
        let (app, _) = get_test_app!();
        let req = test::TestRequest::get()
            .uri("/api/packages/1/patches")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body = test::read_body(resp).await;
        let parsed: Vec<PackagePatchResponse> = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].package_id, 1);
        assert_eq!(parsed[0].url, "http://test.com/patch");
        assert_eq!(parsed[0].sha_512, Some("sha".to_string()));
    }

    #[actix_web::test]
    async fn test_post_patches() {
        let (app, state) = get_test_app!();
        let req = test::TestRequest::post()
            .uri("/api/packages/1/patches")
            .set_json(CreatePackagePatchPayload {
                url: "http://created.com".to_string(),
                sha_512: Some("sha512".to_string()),
            })
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body = test::read_body(resp).await;
        let parsed: PackagePatchResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.package_id, 1);
        assert_eq!(parsed.url, "http://created.com");
        assert_eq!(parsed.sha_512, Some("sha512".to_string()));

        let patches =
            state.orchestrator
                .write()
                .await
                .get_package_store()
                .get_patches_for_package(1)
                .await
                .unwrap();
        assert_eq!(patches.len(), 2);
    }

    #[actix_web::test]
    async fn test_delete_patches() {
        let (app, state) = get_test_app!();
        let req = test::TestRequest::delete()
            .uri("/api/packages/1/patches/1")
            .set_json(CreatePackagePatchPayload {
                url: "http://created.com".to_string(),
                sha_512: Some("sha512".to_string()),
            })
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body = test::read_body(resp).await;
        let parsed: SuccessResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.success, true);

        let patches =
            state.orchestrator
                .write()
                .await
                .get_package_store()
                .get_patches_for_package(1)
                .await
                .unwrap();
        assert_eq!(patches.len(), 0);
    }
}