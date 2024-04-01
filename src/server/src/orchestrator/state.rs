use std::fs::File;
use std::io::Write;
use chrono::{DateTime, Duration, Utc};
use serde_json::Result;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use common::models::PackageStatus;
use crate::models::config::Config;
use crate::models::server_package::ServerPackage;

pub struct PackageBuildData {
    pub time: Option<DateTime<Utc>>,
    pub version: Option<String>,
    pub package_files: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct State {
    packages: Vec<ServerPackage>,

    // Ignore during serialization / des
    path: String
}

impl State {
    pub fn new(config: &Config) -> Self {
        let mut packages: Vec<ServerPackage> = Vec::new();

        for package in config.packages.iter() {
            let package = ServerPackage::from_package_config(package);
            if packages.iter().find(|it| it.get_package_name() == package.get_package_name()).is_some() {
                warn!("Found duplicate package entry for {}, skipping ...", package.get_package_name());
                continue;
            }
            debug!("Loaded package {}", package.get_package_name());
            packages.push(package);
        }

        State {
            packages,
            path: format!("{}/state.json", config.get_serve_path())
        }
    }

    pub fn restore(config: &Config) -> Self {
        let mut state = State::new(config);

        let path = format!("{}/state.json", config.get_serve_path());
        let file = File::open(path.as_str());
        if let Ok(file) = file {
            let deserialized: Result<State> = serde_json::from_reader(file);
            if let Ok(d) = deserialized {
                for package in d.packages.iter() {
                    if let Some(state_package) = state.get_packages_mut().iter_mut()
                        .find(|it| it.get_package_name() == package.get_package_name()) {
                        info!("Restoring state for {}", package.get_package_name());
                        state_package.restore_state(package);
                    } else {
                        info!("Not restoring data for {}: Not found in config", package.get_package_name());
                    }
                }
            }
        }

        state
    }

    pub fn save_to_disk(&self) {
        let file = File::create(self.path.as_str());
        if let Ok(file) = file {
            let res = serde_json::to_writer(file, self);
            if let Err(e) = res {
                error!("Failed to serialize state: {:?}", e);
            }
        } else {
            error!("Failed to open file '{}': {:?}", self.path, file.unwrap_err())
        }
    }

    fn get_mut_package_by_name(&mut self, package_name: &String) -> Option<&mut ServerPackage>
    {
        self.packages.iter_mut().find(|it| it.get_package_name() == package_name)
    }

    pub fn get_next_pending_package(&mut self) -> Option<&mut ServerPackage>
    {
        self.packages.iter_mut().find(|it| it.status == PackageStatus::PENDING)
    }

    pub fn get_packages(&self) -> &Vec<ServerPackage>
    {
        &self.packages
    }

    pub fn get_packages_mut(&mut self) -> &mut Vec<ServerPackage>
    {
        &mut self.packages
    }

    pub fn set_package_status(&mut self, package_name: &String, status: PackageStatus) {
        if let Some(package) = self.get_mut_package_by_name(package_name) {
            package.set_status(status);
        }
        self.save_to_disk();
    }

    pub fn set_package_build_data(
        &mut self,
        package_name: &String,
        build_data: PackageBuildData
    ) {
        if let Some(package) = self.get_mut_package_by_name(package_name) {
            package.last_built = build_data.time;
            package.package.last_built_version = build_data.version;
            package.files = build_data.package_files;
        }
        self.save_to_disk();
    }

    pub fn mark_package_for_rebuild(&mut self, rebuild_interval: u64)
    {
        for package in self.packages.iter_mut()
            .filter(|it| it.status == PackageStatus::FAILED && it.status == PackageStatus::BUILT)
        {
            if let Some(last_built) = package.last_built {
                if Utc::now().signed_duration_since(last_built) > Duration::try_seconds(rebuild_interval as i64).unwrap() {
                    info!("Scheduled rebuild of package {}", package.get_package_name());
                    package.set_status(PackageStatus::PENDING);
                }
            }
        }
        self.save_to_disk();
    }
}