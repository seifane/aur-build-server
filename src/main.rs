mod package_manager;
mod config;
mod utils;
mod args;
mod http;
mod errors;

#[macro_use] extern crate log;
extern crate simplelog;

use std::fs::File;
use simplelog::*;
use clap::Parser;
use crate::args::Args;

use crate::http::server::start_web;
use crate::utils::aurweb::get_package_data;
use crate::utils::package_data::{insert_package, print_dep_tree};
use crate::utils::parse_log_level_from_string;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args: Args = Args::parse();
    let level_filter = parse_log_level_from_string(args.log_level);
    CombinedLogger::init(
        vec![
            TermLogger::new(level_filter, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(level_filter, Config::default(), File::create(args.log_path).unwrap()),
        ]
    ).unwrap();

    start_web().await
}