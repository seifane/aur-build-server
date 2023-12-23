mod api;
mod args;
mod utils;

use clap::Parser;
use colored::Colorize;
use crate::api::Api;
use crate::args::{Args, Commands, PackageCommands};
use crate::utils::package_status_to_colored_string;

fn packages_list(api: &Api) {
    let packages_res = api.get_packages().unwrap();

    for i in packages_res.iter() {
        let mut formatted_date = String::new();
        if let Some(datetime) = i.last_built {
            formatted_date = datetime.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string();
        }

        println!(
            "{} [{}] {} {}",
            i.name.bold(),
            package_status_to_colored_string(&i.status),
            i.last_built_version.as_ref().unwrap_or(&"".to_string()).blue().bold(),
            formatted_date
        );
    }
}

fn packages_rebuild(api: &Api, packages: Vec<String>) {
    let res = api.rebuild_packages(packages);
    match res {
        Ok(res) => {
            if res.success {
                println!("Started rebuilding packages.")
            } else {
                println!("Failed to rebuild packages.");
            }
        }
        Err(e) => {
            println!("Error while rebuilding packages {:?}", e)
        }
    }
}

fn logs_get(api: &Api, package: String, log_type: String) {
    let res = api.get_logs(&package, &log_type);
    match res {
        Ok(contents) => {
            println!("Logs for {} {}", package, log_type);
            println!("------------------------------------");
            println!("{}", contents);
        }
        Err(err) => {
            println!("Failed to fetch logs with error {:?}", err);
        }
    }
}

fn main() {
    let args = Args::parse();

    let api = Api::new(args.host, args.key).unwrap();

    match args.command {
        Commands::Packages { command } => {
            match command {
                PackageCommands::List { .. } => packages_list(&api),
                PackageCommands::Rebuild { packages } => packages_rebuild(&api, packages),
            }
        }
        Commands::Logs { package, log_type } => logs_get(&api, package, log_type)
    }
}