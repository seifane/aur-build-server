mod package_manager;
mod config;
mod utils;
mod args;

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use args::Args;
use actix_web::{App, HttpServer, get, web, HttpResponse, HttpRequest, middleware};
use actix_web::dev::{Service, ServiceRequest};
use actix_web::error::ErrorUnauthorized;
use actix_web::http::header::{AUTHORIZATION, CONTENT_TYPE};
use actix_web::http::{Error, HeaderValue, StatusCode};
use actix_web::web::Query;
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use clap::Parser;
use serde::{Serialize};


use package_manager::PackageManager;
use crate::config::load_config;
use crate::package_manager::Package;

#[derive(Serialize)]
struct BasicResultResponse {
    pub ok: bool,
}

#[derive(Serialize)]
struct PackagesResponse<'a> {
    pub is_running: bool,
    pub commit_queued: bool,
    pub packages: &'a Vec<Package>,
}

#[get("/api/packages/rebuild/{package_name}")]
async fn api_build_repo(data: web::Data<Mutex<PackageManager>>, req: HttpRequest) -> HttpResponse {
    let mut package_manager = data.lock().unwrap();

    let package_name: Option<&str> = req.match_info().get("package_name");

    if package_name.is_some() {
        package_manager.rebuild_package(package_name.unwrap().to_string());
    } else {
        package_manager.rebuild_packages();
    }

    package_manager.queue_commit();
    HttpResponse::Ok().json(BasicResultResponse {
        ok: true
    })
}

#[get("/api/packages")]
async fn api_get_queue_finished(data: web::Data<Mutex<PackageManager>>) -> HttpResponse {
    let package_manager = data.lock().expect("package_manager");
    let x = package_manager.packages.lock().unwrap();
    HttpResponse::Ok().json(PackagesResponse {
        is_running: package_manager.is_running.load(Ordering::SeqCst),
        commit_queued: package_manager.commit_queued.load(Ordering::SeqCst),
        packages: x.deref(),
    })
}

#[get("/api/commit")]
async fn api_commit(data: web::Data<Mutex<PackageManager>>) -> HttpResponse {
    let mut package_manager = data.lock().unwrap();
    package_manager.queue_commit();
    HttpResponse::Ok().json(BasicResultResponse {
        ok: true
    })
}

#[get("/api/start")]
async fn api_start(data: web::Data<Mutex<PackageManager>>) -> HttpResponse {
    let mut package_manager = data.lock().unwrap();
    package_manager.start_workers();
    HttpResponse::Ok().json(BasicResultResponse {
        ok: true
    })
}

#[get("/api/stop")]
async fn api_stop(data: web::Data<Mutex<PackageManager>>) -> HttpResponse {
    let mut package_manager = data.lock().unwrap();
    package_manager.stop_workers();
    HttpResponse::Ok().json(BasicResultResponse {
        ok: true
    })
}

async fn validator(req: ServiceRequest, credentials: BearerAuth) -> Result<ServiceRequest, actix_web::Error> {
    let args: Args = Args::parse();
    let config = load_config(args.config_path);
    if config.apikey.is_some() {
        println!("api key is some");
        if config.apikey.unwrap() == credentials.token().to_string() {
            println!("api key is good");
            return Ok(req);
        } else {
            println!("api key is shit");
            return Err(ErrorUnauthorized("Unauthorized"));
        }
    }
    println!("api key is none");
    Ok(req)
}

async fn start_web() -> std::io::Result<()> {
    let args: Args = Args::parse();
    let config = load_config(args.config_path);

    let bind_addr = format!("0.0.0.0:{}", args.port);

    let data = web::Data::new(Mutex::new(PackageManager::new(config.clone())));
    data.lock().unwrap().start_workers();
    data.lock().unwrap().load_packages();
    data.lock().unwrap().queue_commit();

    println!("Starting server on {} ...", bind_addr);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(middleware::Logger::default())
            .wrap(HttpAuthentication::bearer(validator))
            // Hacky way to allow requests with no tokens
            .wrap_fn(|mut req, srv| {
                if !req.headers().contains_key("Authorization") {
                    req.headers_mut().insert(
                        AUTHORIZATION, HeaderValue::from_static("Bearer default")
                    )
                }
                let fut = srv.call(req);
                async {
                    let mut res = fut.await?;
                    Ok(res)
                }
            })
            .service(api_build_repo)
            .service(api_get_queue_finished)
            .service(api_commit)
            .service(actix_files::Files::new("/", "serve").show_files_listing())
    }).bind(bind_addr)?.run().await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    start_web().await
}