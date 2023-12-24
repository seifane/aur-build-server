use colored::Colorize;
use dialoguer::Input;
use dialoguer::theme::ColorfulTheme;
use crate::api::Api;
use crate::profile::{Profile, ProfileConfig};
use crate::utils::package_status_to_colored_string;

pub fn packages_list(api: &Api) {
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

pub fn packages_rebuild(api: &Api, packages: Vec<String>) {
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

pub fn logs_get(api: &Api, package: String, log_type: String) {
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
        api_key
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