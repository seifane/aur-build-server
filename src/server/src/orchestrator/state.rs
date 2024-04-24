use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use chrono::{DateTime, Duration, Utc};
use log::{debug, error, info, warn};
use common::models::PackageStatus;
use crate::http::util::MultipartField;
use crate::models::config::Config;
use crate::models::package_state::PackageState;
use crate::models::server_package::{ServerPackage};

pub struct PackageBuildData<'a> {
    pub files: Option<&'a Vec<MultipartField>>,
    pub log_files: Option<&'a Vec<MultipartField>>,

    pub errors: Vec<String>,
    pub time: Option<DateTime<Utc>>,
    pub version: Option<String>,
}

pub struct State {
    packages: HashMap<String, ServerPackage>,
    path: String
}

impl State {
    pub fn new(config: &Config) -> Self {
        let mut packages: HashMap<String, ServerPackage> = HashMap::new();

        for package in config.packages.iter() {
            if packages.contains_key(&package.name) {
                warn!("Found duplicate package entry for {}, skipping ...", package.name);
                continue;
            }

            let server_package = ServerPackage::from_package_definition(package.clone());
            packages.insert(server_package.get_package_name().clone(), server_package);
            debug!("Loaded package {}", package.name);
        }

        State {
            packages,
            path: config.get_serve_path()
        }
    }

    pub fn restore(&mut self) -> Result<(), Box<dyn Error>> {
        let path = format!("{}/packages_state.json", self.path);
        let file = File::open(path.as_str())?;
        let package_states: HashMap<String, PackageState> = serde_json::from_reader(file)?;

        for (package_name, state) in package_states.iter() {
            if let Some(package) = self.packages.get_mut(package_name) {
                info!("Restoring state for {}", package.get_package_name());
                package.state = state.clone();
            } else {
                warn!("Dropping state for {}: not found in config", package_name);
            }
        }

        Ok(())
    }

    pub fn save_to_disk(&self) {
        let path = format!("{}/packages_state.json", self.path);
        let file = File::create(path.as_str());
        if let Ok(file) = file {
            let states: HashMap<String, PackageState> = self.packages
                .iter()
                .map(|(name, package)| (name.clone(), package.state.clone())).collect();
            let res = serde_json::to_writer(file, &states);
            if let Err(e) = res {
                error!("Failed to serialize state: {:?}", e);
            }
        } else {
            error!("Failed to open file '{}': {:?}", self.path, file.unwrap_err())
        }
    }

    pub fn get_package_by_name(&self, package_name: &String) -> Option<&ServerPackage> {
        self.packages.get(package_name)
    }

    fn get_mut_package_by_name(&mut self, package_name: &String) -> Option<&mut ServerPackage>
    {
        self.packages.get_mut(package_name)
    }

    pub fn get_next_pending_package(&mut self) -> Option<&mut ServerPackage>
    {
        self.packages
            .iter_mut()
            .find(|(_, package)| package.state.status == PackageStatus::PENDING)
            .map(|it| it.1)
    }

    pub fn get_packages(&self) -> &HashMap<String, ServerPackage>
    {
        &self.packages
    }

    pub fn queue_package_for_rebuild(&mut self, package_name: &String, force: bool) {
        if let Some(package) = self.get_mut_package_by_name(package_name) {
            package.set_status(PackageStatus::PENDING);
            if force {
                package.clear_last_built_version();
            }
        }
        self.save_to_disk();
    }

    pub fn queue_all_packages_for_rebuild(&mut self, force: bool) {
        for (_, package) in self.packages.iter_mut() {
            if package.state.status != PackageStatus::BUILDING {
                package.set_status(PackageStatus::PENDING);
                if force {
                    package.clear_last_built_version();
                }
            }
        }
        self.save_to_disk();
    }

    pub fn set_package_status(&mut self, package_name: &String, status: PackageStatus) {
        if let Some(package) = self.get_mut_package_by_name(package_name) {
            package.set_status(status);
        }
        self.save_to_disk();
    }

    pub fn update_package_state_from_build_data(
        &mut self,
        package_name: &String,
        build_data: PackageBuildData,
        package_files: Vec<String>
    ) -> bool {
        let mut should_notify = false;

        if let Some(package) = self.get_mut_package_by_name(package_name) {
            if let Some(error) = build_data.errors.first() {
                package.state.status = PackageStatus::FAILED;
                package.state.last_error = Some(error.clone());
                error!("Error while building {}: {}", package_name, error);
            } else {
                package.state.status = PackageStatus::BUILT;
                package.state.last_error = None;
                info!("Built {}", package_name);
            }

            if package.state.last_built_version == build_data.version || package.state.status == PackageStatus::FAILED {
                should_notify = true;
            }
            package.state.last_built = build_data.time;
            package.state.last_built_version = build_data.version;
            package.state.files = package_files;
        }
        self.save_to_disk();

        should_notify
    }

    pub fn mark_package_for_rebuild(&mut self, rebuild_interval: u64)
    {
        for (_, package) in self.packages.iter_mut()
            .filter(|(_, it)| it.state.status == PackageStatus::FAILED || it.state.status == PackageStatus::BUILT)
        {
            if let Some(last_built) = package.state.last_built {
                if Utc::now().signed_duration_since(last_built) > Duration::try_seconds(rebuild_interval as i64).unwrap() {
                    info!("Scheduled rebuild of package {}", package.get_package_name());
                    package.set_status(PackageStatus::PENDING);
                }
            }
        }
        self.save_to_disk();
    }
}