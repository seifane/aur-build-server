mod package_manager;
mod config;
mod utils;
mod args;
mod http;
mod errors;

use crate::http::server::start_web;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    start_web().await
}