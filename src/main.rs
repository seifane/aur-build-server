mod package_manager;
mod config;
mod utils;
mod args;
mod http;

use std::ops::Deref;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use args::Args;
use actix_web::{App, HttpServer, get, web, HttpResponse, HttpRequest, middleware};
use clap::Parser;
use serde::{Serialize};
use http::Auth;


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
            .wrap(Auth {apikey: config.apikey.clone()})
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