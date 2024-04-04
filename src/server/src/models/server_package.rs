use log::info;
use serde::{Deserialize, Serialize};
use common::http::responses::{PackageResponse};
use common::models::{PackageDefinition, PackageJob, PackageStatus};
use crate::models::package_state::PackageState;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerPackage {
    package: PackageDefinition,
    pub state: PackageState,
}

impl ServerPackage {
    pub fn from_package_definition(package_config: PackageDefinition) -> ServerPackage {
        ServerPackage {
            package: package_config,
            state: PackageState::new(),
        }
    }

    pub fn get_package_name(&self) -> &String {
        &self.package.name
    }

    pub fn set_status(&mut self, status: PackageStatus) {
        info!("Package {} status changed to {:?}", self.package.name, status);
        self.state.status = status;
    }

    pub fn get_state(&self) -> &PackageState
    {
        &self.state
    }

    pub fn set_state(&mut self, state: PackageState)
    {
        self.state = state;
    }

    pub fn get_response(&self) -> PackageResponse
    {
        PackageResponse {
            package: self.package.clone(),
            status: self.state.status,
            last_built: self.state.last_built.clone(),
            last_built_version: self.state.last_built_version.clone(),
        }
    }

    pub fn get_package_job(&self) -> PackageJob {
        PackageJob {
            definition: self.package.clone(),
            last_built_version: self.state.last_built_version.clone(),
        }
    }
}