use cli_table::{Cell, Style, Table};
use colored::Colorize;
use dialoguer::Input;
use dialoguer::theme::ColorfulTheme;
use crate::api::Api;
use crate::profile::{Profile, ProfileConfig};
use crate::utils::{get_color_from_package_status, get_color_from_worker_status};

pub fn workers_list(api: &Api) {
    let workers_res = api.get_workers().unwrap();
    println!("Workers");
    let mut rows = Vec::new();
    for worker in workers_res.iter() {
        rows.push(vec![
            worker.id.cell(),
            worker.status.to_string().cell().foreground_color(get_color_from_worker_status(&worker.status)),
            worker.current_job.as_ref().unwrap_or(&"None".to_string()).as_str().cell(),
        ]);
    }
    println!("{}", rows.table()
        .title(vec![
            "ID".cell().bold(true),
            "Status".cell().bold(true),
            "Current Job".cell().bold(true),
        ])
        .display()
        .unwrap())
}

pub fn workers_delete(api: &Api, id: usize) {
    let res = api.delete_worker(id).unwrap();
    if res.success {
        println!("Evicted worker successfully");
    } else {
        println!("Failed to evict worker, is the id correct ?");
    }
}

pub fn packages_list(api: &Api) {
    let packages_res = api.get_packages().unwrap();

    let mut rows = Vec::new();
    for package in packages_res.iter() {
        let mut last_built_date = "Never".to_string();
        if let Some(datetime) = package.last_built {
            last_built_date = datetime.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string();
        }

        rows.push(vec![
            (&package.package.name).cell(),
            package.status.to_string().cell().foreground_color(get_color_from_package_status(&package.status)),
            package.last_built_version.as_ref().unwrap_or(&"None".to_string()).cell(),
            last_built_date.cell(),
            package.last_error.as_ref().unwrap_or(&"None".to_string()).cell(),
        ]);

    }
    println!("{}", rows.table()
        .title(vec![
            "Name".cell().bold(true),
            "Status".cell().bold(true),
            "Last Built Version".cell().bold(true),
            "Last Built Date".cell().bold(true),
            "Last Error".cell().bold(true)
        ])
        .display()
        .unwrap());
}

pub fn packages_rebuild(api: &Api, packages: Vec<String>, force: bool) {
    let res = api.rebuild_packages(packages, force);
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

pub fn logs_get(api: &Api, package: String) {
    let res = api.get_logs(&package);
    match res {
        Ok(contents) => {
            println!("Logs for {package}");
            println!("{}", contents);
        }
        Err(err) => {
            println!("Failed to fetch logs with error {:?}", err);
        }
    }
}

pub fn webhook_trigger_package_update(api: &Api, package: &String) {
    let res = api.webhook_trigger_package(package);
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
        println!("- {} | {} {}", profile.name.bold(), profile.base_url, default_text)
    }
}

pub fn profile_set_default(config: &mut ProfileConfig, name: &String)
{
    if let Err(err) = config.set_default_profile(name) {
        println!("Unable to set default profile: {}", err);
    }

    config.save_to_file().expect("Failed to save config file");

    println!("Default profile set");
}