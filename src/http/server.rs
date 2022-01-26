use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use actix_web::{App, HttpServer, web, HttpResponse, HttpRequest, Responder};
use clap::Parser;

use crate::Args;
use crate::config::load_config;
use crate::http::responses::{BasicResultResponse, PackagesResponse};
use crate::http::auth_middleware::Auth;
use crate::package_manager::PackageManager;


async fn api_build_repo(data: web::Data<Mutex<PackageManager>>, req: HttpRequest) -> impl Responder {
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

async fn api_get_packages(data: web::Data<Mutex<PackageManager>>) -> impl Responder {
    let package_manager = data.lock().expect("package_manager");
    let x = package_manager.packages.lock().unwrap();
    HttpResponse::Ok().json(PackagesResponse {
        is_running: package_manager.is_running.load(Ordering::SeqCst),
        commit_queued: package_manager.commit_queued.load(Ordering::SeqCst),
        packages: x.deref(),
    })
}

async fn api_commit(data: web::Data<Mutex<PackageManager>>) -> HttpResponse {
    let mut package_manager = data.lock().unwrap();
    package_manager.queue_commit();
    HttpResponse::Ok().json(BasicResultResponse {
        ok: true
    })
}

async fn api_start(data: web::Data<Mutex<PackageManager>>) -> HttpResponse {
    let mut package_manager = data.lock().unwrap();
    package_manager.start_workers();
    HttpResponse::Ok().json(BasicResultResponse {
        ok: true
    })
}

async fn api_stop(data: web::Data<Mutex<PackageManager>>) -> HttpResponse {
    let mut package_manager = data.lock().unwrap();
    package_manager.stop_workers();
    HttpResponse::Ok().json(BasicResultResponse {
        ok: true
    })
}

pub async fn start_web() -> std::io::Result<()> {
    let args: Args = Args::parse();
    let config = load_config(args.config_path);

    let bind_addr = format!("0.0.0.0:{}", args.port);

    let data = web::Data::new(Mutex::new(PackageManager::new(config.clone())));
    data.lock().unwrap().start_workers();
    data.lock().unwrap().load_packages();
    data.lock().unwrap().queue_commit();

    let serve_path = Path::new("./serve");
    if !serve_path.exists() {
        fs::create_dir(serve_path).unwrap();
    }

    println!("Starting server on {} ...", bind_addr);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .service(actix_files::Files::new("/repo", "serve").show_files_listing())
            .service(
                web::scope("/api")
                    .service(web::resource("/start").to(api_start))
                    .service(web::resource("/stop").to(api_stop))
                    .service(web::resource("/commit").to(api_commit))
                    .service(web::resource("/packages").to(api_get_packages))
                    .service(web::resource("/packages/rebuild").to(api_build_repo))
                    .service(web::resource("/packages/rebuild/{package_name}").to(api_build_repo))
                    .wrap(Auth {apikey: config.apikey.clone()})
            )
    }).bind(bind_addr)?.run().await
}