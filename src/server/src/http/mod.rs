mod api_worker;
mod base;
mod packages;
mod workers;

use crate::models::config::Config;
use crate::orchestrator::Orchestrator;
use actix_web::web::ServiceConfig;
use actix_web::{web, App, HttpServer};
use anyhow::Result;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::RwLock;

// async fn authorize((token, headers): (String, HeaderMap<HeaderValue>)) -> Result<(), Rejection> {
//     return match headers.get("authorization") {
//         Some(authorization) => {
//             let auth = from_utf8(authorization.as_bytes()).unwrap();
//             if auth == token.as_str() {
//                 Ok(())
//             } else {
//                 Err(reject::reject())
//             }
//         }
//         None => Err(reject::reject()),
//     }
// }
//
// pub fn with_auth(token: String) -> impl Filter<Extract=((), ), Error=Rejection> + Clone {
//     headers_cloned()
//         .map(move |headers: HeaderMap<HeaderValue>| (token.clone(), headers))
//         .and_then(authorize)
// }

#[derive(Clone)]
pub struct HttpState {
    pub orchestrator: Arc<RwLock<Orchestrator>>,
    pub config: Arc<RwLock<Config>>,
}

fn get_app(cfg: &mut ServiceConfig, state: HttpState) {
    cfg
        .app_data(web::Data::new(state.clone()))
        .service(
            web::scope("/api")
                .service(workers::register())
                .service(packages::register()),
        )
        .service(api_worker::register());
}

pub async fn start_http(state: HttpState) -> Result<()> {
    let addr = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        state.config.read().await.port,
    );

    HttpServer::new(move || {
        App::new()
            .configure(|cfg| get_app(cfg, state.clone()))
    })
    .bind(addr)?
    .run()
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[macro_export]
    macro_rules! get_test_app {
        () => {
            {
                use crate::http::{get_app, HttpState};
                use crate::models::config::Config;
                use crate::orchestrator::Orchestrator;
                use crate::persistence::package_store::PackageInsert;
                use actix_web::{test, App};
                use log::LevelFilter;
                use std::path::PathBuf;
                use std::sync::Arc;
                use tokio::sync::RwLock;
                use chrono::Utc;


                 let config = Config {
                    log_level: LevelFilter::Off,
                    log_path: PathBuf::from("/tmp/aur-build-server-test/log.txt"),
                    api_key: "api_key".to_string(),
                    port: 3000,
                    repo_name: "test".to_string(),
                    sign_key: None,
                    rebuild_time: None,
                    serve_path: PathBuf::from("/tmp/aur-build-server-test/repo"),
                    build_logs_path: PathBuf::from("/tmp/aur-build-server-test/logs"),
                    database_path: ":memory:".into(),
                    webhooks: vec![],
                    packages: vec![],
                };
                let config = Arc::new(RwLock::new(config));
                let mut orchestrator = Orchestrator::new(config.clone()).await.unwrap();

                orchestrator.get_package_store().create_package(PackageInsert {
                    name: "first".to_string(),
                    run_before: None,
                }).await.unwrap();
                let mut package = orchestrator.get_package_store().create_package(PackageInsert {
                    name: "second".to_string(),
                    run_before: Some("run_before_second".to_string()),
                }).await.unwrap();
                package.set_status(PackageStatus::BUILT);
                package.last_built_version = Some("lastver".to_string());
                package.get_files_mut().push("file1.tar".to_string());
                package.set_last_built(Some(Utc::now()));
                orchestrator.get_package_store().update_package(&package).await.unwrap();

                let state = HttpState {
                    orchestrator: Arc::new(RwLock::new(orchestrator)),
                    config,
                };
                let app = App::new()
                    .configure(|cfg| get_app(cfg, state.clone()));

                    (test::init_service(app).await, state)
                }
        };
    }
}
