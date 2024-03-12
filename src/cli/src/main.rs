mod api;
mod args;
mod utils;
mod commands;
mod profile;

use clap::Parser;
use colored::Colorize;
use crate::api::Api;
use crate::args::{Args, Commands, PackageCommands, ProfileCommands};
use crate::commands::{logs_get, packages_list, packages_rebuild, profile_create, profile_delete, profile_list, profile_set_default};
use crate::profile::ProfileConfig;

fn main() {
    let args = Args::parse();

    let mut profile_config = ProfileConfig::from_file().expect("Unable to load profile config");

    let api = if args.base_url.is_some() && args.api_key.is_some() {
        Api::new(args.base_url.unwrap(), args.api_key.unwrap()).unwrap()
    } else {
        let profile = if let Some(profile_name) = args.profile {
            let profile = profile_config.get_profile_by_name(&profile_name);
            if profile.is_none() {
                println!("Profile {} not found", profile_name);
                return;
            }
            profile.unwrap()
        } else {
            let profile = profile_config.get_default_profile();
            if profile.is_none() {
                println!("No default profile found. Create one with profiles create.");
                return;
            }

            profile.unwrap()
        };
        println!("Using profile {}\n", profile.name.bold());
        Api::new(profile.base_url.clone(), profile.api_key.clone()).unwrap()
    };

    match args.command {
        Commands::Packages { command } => {
            match command {
                PackageCommands::List {} => packages_list(&api),
                PackageCommands::Rebuild { packages } => packages_rebuild(&api, packages),
            }
        }
        Commands::Logs { package} => logs_get(&api, package),
        Commands::Profiles { command } => {
            match command {
                ProfileCommands::List {} => profile_list(&profile_config),
                ProfileCommands::Create {} => profile_create(&mut profile_config),
                // ProfileCommands::Update { .. } => {}
                ProfileCommands::Delete { name } => profile_delete(&mut profile_config, &name),
                ProfileCommands::SetDefault { name } => profile_set_default(&mut profile_config, &name)
            }
        }
    }
}

