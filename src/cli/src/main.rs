mod api;
mod args;
mod utils;
mod commands;
mod profile;

use std::process::exit;
use clap::Parser;
use colored::Colorize;
use crate::api::Api;
use crate::args::{Args, Commands, PackageCommands, PatchCommands, ProfileCommands, WebhookCommands, WebhookTriggerCommands, WorkerCommands};
use crate::commands::{logs_get, packages_create, packages_delete, packages_list, packages_rebuild, patches_create, patches_delete, patches_list, profile_create, profile_delete, profile_list, profile_set_default, webhook_trigger_package_update, workers_delete, workers_list};
use crate::profile::ProfileConfig;

fn get_api(args: &Args, profile_config: &ProfileConfig) -> Api {
    let api = if args.base_url.is_some() && args.api_key.is_some() {
        Api::new(args.base_url.as_ref().unwrap().clone(), args.api_key.as_ref().unwrap().clone()).unwrap()
    } else {
        let profile = if let Some(profile_name) = args.profile.as_ref() {
            let profile = profile_config.get_profile_by_name(&profile_name);
            if profile.is_none() {
                println!("Profile {} not found", profile_name);
                exit(1);
            }
            profile.unwrap()
        } else {
            let profile = profile_config.get_default_profile();
            if profile.is_none() {
                println!("No default profile found. Create one with profiles create.");
                exit(1);
            }

            profile.unwrap()
        };
        println!("Using profile {}", profile.name.bold());
        Api::new(profile.base_url.clone(), profile.api_key.clone()).unwrap()
    };

    api
}

fn main() {
    let args = Args::parse();

    let mut profile_config = ProfileConfig::from_file().expect("Unable to load profile config");
    let api = get_api(&args, &profile_config);

    match &args.command {
        Commands::Workers { command} => {
            match command {
                WorkerCommands::List { .. } => workers_list(&api),
                WorkerCommands::Evict { id } => workers_delete(&api, *id)
            }
        },
        Commands::Packages { command } => {
            match command {
                PackageCommands::List {} => packages_list(&api),
                PackageCommands::Create { name, run_before} => packages_create(&api, name, run_before),
                PackageCommands::Delete { name } => packages_delete(&api, name),
                PackageCommands::Rebuild { packages, force } => packages_rebuild(&api, packages.clone(), *force),
            }
        }
        Commands::Patches { command } => {
            match command {
                PatchCommands::List { package_name } => patches_list(&api, package_name),
                PatchCommands::Create { package_name, url, sha_512 } =>
                    patches_create(&api, package_name, url, sha_512),
                PatchCommands::Delete { package_name, id } =>
                    patches_delete(&api, package_name, *id)
            }
        }
        Commands::Logs { package} => logs_get(&api, package.clone()),
        Commands::Webhooks {command} => {
            match command {
                WebhookCommands::Trigger { command } => {
                    match command {
                        WebhookTriggerCommands::PackageUpdated { package_name } => {
                            webhook_trigger_package_update(&get_api(&args, &profile_config), package_name);
                        }
                    }
                }
            }
        }
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

