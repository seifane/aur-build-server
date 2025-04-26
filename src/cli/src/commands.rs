use crate::api::Api;
use crate::profile::{Profile, ProfileConfig};
use crate::utils::{get_color_from_package_status, get_color_from_worker_status};
use chrono::Local;
use cli_table::{Cell, CellStruct, Style, Table};
use colored::Colorize;
use common::http::payloads::CreatePackagePatchPayload;
use common::models::PackageStatus;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input};
use std::collections::HashMap;

macro_rules! try_get_package_from_name {
    ($api:expr, $package:expr) => {
        match $api.get_package_from_name($package) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Failed to get package from name: {}", e);
                return;
            }
        }
    };
}

pub fn workers_list(api: &Api) {
    let workers_res = api.get_workers().unwrap();
    println!("Workers");
    let mut rows = Vec::new();
    for worker in workers_res.iter() {
        rows.push(vec![
            worker.id.cell(),
            worker
                .status
                .to_string()
                .cell()
                .foreground_color(Some(get_color_from_worker_status(&worker.status).into())),
            worker
                .current_job
                .as_ref()
                .unwrap_or(&"None".to_string())
                .as_str()
                .cell(),
        ]);
    }
    println!(
        "{}",
        rows.table()
            .title(vec![
                "ID".cell().bold(true),
                "Status".cell().bold(true),
                "Current Job".cell().bold(true),
            ])
            .display()
            .unwrap()
    )
}

pub fn workers_delete(api: &Api, id: usize) {
    let res = api.delete_worker(id).unwrap();
    if res.success {
        println!("Evicted worker successfully");
    } else {
        eprintln!("Failed to evict worker, is the id correct ?");
    }
}

pub fn packages_list(api: &Api, compact: &bool) {
    let packages_res = api.get_packages().unwrap();

    let mut status_counts = HashMap::new();

    let rows: Vec<Vec<_>> = packages_res
        .into_iter()
        .map(|package| {
            if let Some(count) = status_counts.get_mut(&package.status) {
                *count += 1;
            } else {
                status_counts.insert(package.status, 1);
            }
            vec![
                package.name,
                package
                    .status
                    .to_string()
                    .color::<colored::Color>(get_color_from_package_status(&package.status).into())
                    .to_string(),
                package
                    .last_built_version
                    .as_ref()
                    .unwrap_or(&"None".to_string())
                    .to_string(),
                package
                    .last_built
                    .map(|dt| {
                        dt.with_timezone(&Local)
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string()
                    })
                    .unwrap_or("Never".to_string()),
            ]
        })
        .collect();

    if *compact {
        for package in rows.iter() {
            println!(
                "{} | {} | {} | {}",
                package[0], package[1], package[2], package[3],
            );
        }
    } else {
        println!(
            "{}",
            rows.into_iter()
                .map(|v| v.into_iter().map(|s| s.cell()))
                .table()
                .title(vec![
                    "Name".cell().bold(true),
                    "Status".cell().bold(true),
                    "Last Built Version".cell().bold(true),
                    "Last Built Date".cell().bold(true),
                ])
                .display()
                .unwrap()
        );
    }

    println!(
        "\nPending: {}, Building: {}, Built: {}, Failed: {}",
        status_counts.get(&PackageStatus::PENDING).unwrap_or(&0),
        status_counts.get(&PackageStatus::BUILDING).unwrap_or(&0),
        status_counts.get(&PackageStatus::BUILT).unwrap_or(&0),
        status_counts.get(&PackageStatus::FAILED).unwrap_or(&0),
    );
}

pub fn packages_get(api: &Api, name: &String) {
    let package = try_get_package_from_name!(api, name);

    println!("ID: {}", package.id);
    println!("Name: {}", package.name);
    println!("Run Before Command: {:?}", package.run_before);
    println!("Status: {}", package.status.to_string());
    println!(
        "Last Built: {}",
        package
            .last_built
            .map(|dt| dt
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string())
            .unwrap_or("Never".to_string())
    );
    println!("Files: {:?}", package.files);
    println!("Last Built Version {:?}", package.last_built_version);
    println!("Last Error {:?}", package.last_error);
}

pub fn packages_create(api: &Api, name: &Option<String>, run_before: &Option<String>) {
    let (name, run_before) = match name.as_ref() {
        None => {
            let name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Package name")
                .interact_text()
                .unwrap();
            let run_before: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Run before command")
                .allow_empty(true)
                .interact_text()
                .unwrap();
            let run_before = if run_before.is_empty() {
                None
            } else {
                Some(run_before)
            };
            (name, run_before)
        }
        Some(name) => (name.to_string(), run_before.clone()),
    };

    match api.create_package(name, run_before) {
        Ok(package) => println!("Package {} created successfully", package.name),
        Err(e) => eprintln!("Failed to create package: {}", e),
    }
}

pub fn packages_delete(api: &Api, name: &String) {
    let package = try_get_package_from_name!(api, name);
    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Do you want to delete {} ?", package.name))
        .default(false)
        .interact()
        .unwrap()
    {
        match api.delete_package(package.id) {
            Ok(_) => println!("Package {} deleted", package.name),
            Err(e) => eprintln!("Failed to delete package {}: {}", package.name, e),
        }
    }
}

pub fn packages_rebuild(api: &Api, packages: Vec<String>, force: bool) {
    let mut package_ids = Vec::new();

    for package in packages {
        package_ids.push(try_get_package_from_name!(api, &package).id);
    }

    let res = api.rebuild_packages(package_ids, force);
    match res {
        Ok(res) => {
            if res.success {
                println!("Started rebuilding packages.")
            } else {
                println!("Failed to rebuild packages.");
            }
        }
        Err(e) => {
            eprintln!("Error while rebuilding packages {:?}", e)
        }
    }
}

pub fn patches_list(api: &Api, package_name: &String) {
    let package = try_get_package_from_name!(api, package_name);

    match api.get_patches(package.id) {
        Ok(patches) => {
            if patches.is_empty() {
                println!("No patches found.");
                return;
            }
            let patches: Vec<Vec<CellStruct>> = patches
                .into_iter()
                .map(|patch| {
                    vec![
                        patch.id.cell(),
                        patch.url.cell(),
                        patch.sha_512.unwrap_or("None".to_string()).cell(),
                    ]
                })
                .collect();
            println!(
                "{}",
                patches
                    .table()
                    .title(vec![
                        "Id".cell().bold(true),
                        "Url".cell().bold(true),
                        "SHA 512".cell().bold(true),
                    ])
                    .display()
                    .unwrap()
            );
        }
        Err(e) => {
            eprintln!("Error while getting patches: {}", e);
        }
    }
}

pub fn patches_create(api: &Api, package_name: &String, url: &String, sha_512: &Option<String>) {
    let package = try_get_package_from_name!(api, package_name);

    match api.create_patch(
        package.id,
        CreatePackagePatchPayload {
            url: url.clone(),
            sha_512: sha_512.clone(),
        },
    ) {
        Ok(patch) => println!("Patch created successfully with id {}", patch.id),
        Err(e) => eprintln!("Failed to create patch: {}", e),
    }
}

pub fn patches_delete(api: &Api, package_name: &String, id: i32) {
    let package = try_get_package_from_name!(api, package_name);

    match api.delete_patch(package.id, id) {
        Ok(_) => println!("Patch {} deleted", id),
        Err(e) => eprintln!("Failed to delete patch: {}", e),
    }
}

pub fn logs_get(api: &Api, package: String) {
    let package = try_get_package_from_name!(api, &package);
    let res = api.get_logs(package.id);
    match res {
        Ok(contents) => {
            println!("Logs for {}", package.name);
            println!("{}", contents);
        }
        Err(err) => {
            println!("Failed to fetch logs with error {:?}", err);
        }
    }
}

pub fn webhook_trigger_package_update(api: &Api) {
    let res = api.webhook_trigger_package();
    match res {
        Ok(response) => {
            if response.success {
                println!("Webhook sent successfully");
            } else {
                println!("Failed to send webhook, check the package name");
            }
        }
        Err(e) => {
            println!("Failed to send webhook: {}", e.to_string())
        }
    }
}

pub fn profile_create(config: &mut ProfileConfig) {
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Profile name")
        .interact_text()
        .unwrap();

    let base_url: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Base URL")
        .interact_text()
        .unwrap();

    let api_key: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("API Key")
        .interact_text()
        .unwrap();

    let res = config.add_profile(Profile {
        name,
        base_url,
        api_key,
    });

    if let Err(err) = res {
        println!("Unable to add profile: {}", err);
        return;
    }

    config.save_to_file().expect("Failed to save config file");

    println!("Profile created");
}

pub fn profile_delete(config: &mut ProfileConfig, name: &String) {
    if let Err(e) = config.remove_profile(name) {
        println!("Unable to remove profile from config: {}", e);
        return;
    }
    config.save_to_file().expect("Failed to save config file");

    println!("Profile removed");
}

pub fn profile_list(config: &ProfileConfig) {
    for profile in config.get_profiles() {
        let default_text = if &profile.name == config.get_default_profile_name() {
            "(Default)".bold().cyan().to_string()
        } else {
            "".to_string()
        };
        println!(
            "- {} | {} {}",
            profile.name.bold(),
            profile.base_url,
            default_text
        )
    }
}

pub fn profile_set_default(config: &mut ProfileConfig, name: &String) {
    if let Err(err) = config.set_default_profile(name) {
        println!("Unable to set default profile: {}", err);
    }

    config.save_to_file().expect("Failed to save config file");

    println!("Default profile set");
}
