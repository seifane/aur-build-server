use std::error::Error;
use reqwest::blocking::Client;
use reqwest::header;
use reqwest::header::{HeaderMap, HeaderValue};
use common::http::payloads::{CreatePackagePatchPayload, CreatePackagePayload, PackageRebuildPayload};
use common::http::responses::{PackagePatchResponse, PackageResponse, SuccessResponse, WorkerResponse};
use anyhow::{anyhow, Result};

pub struct Api {
    client: Client,
    host: String,
}

impl Api {
    pub fn new(host: String, api_key: String) -> Result<Self>
    {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_str(api_key.as_str())?);

        let client = Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Api {
            client,
            host
        })
    }

    pub fn get_package_from_name(&self, package_name: &String) -> Result<PackageResponse>
    {
        let mut found_packages = self.search_package(package_name)?;
        if found_packages.len() > 1 {
            return Err(
                anyhow!(
                    "Package {} is not clear, found: {}",
                    package_name,
                    found_packages.into_iter().map(|p| p.name).collect::<Vec<_>>().join(", ")
                )
            );
        } else if found_packages.is_empty() {
            return Err(anyhow!("Package {} not found", package_name));
        }

        Ok(found_packages.remove(0))
    }

    pub fn search_package(&self, query: &String) -> Result<Vec<PackageResponse>>
    {
        Ok(
            self.client
                .get(format!("{}/api/packages?search={}", self.host, query))
                .send()?
                .json()?
        )
    }

    pub fn get_packages(&self) -> Result<Vec<PackageResponse>>
    {
        Ok(
            self.client
                .get(format!("{}/api/packages", self.host))
                .send()?
                .json()?
        )
    }

    pub fn create_package(&self, name: String, run_before: Option<String>) -> Result<PackageResponse>
    {
        Ok(
            self.client
                .post(format!("{}/api/packages", self.host))
                .json(&CreatePackagePayload {
                    name,
                    run_before,
                })
                .send()?
                .json()?
        )
    }

    pub fn delete_package(&self, id: i32) -> Result<SuccessResponse>
    {
        Ok(
            self.client
                .delete(format!("{}/api/packages/{}", self.host, id))
                .send()?
                .json()?
        )
    }

    pub fn rebuild_packages(&self, packages: Vec<i32>, force: bool) -> Result<SuccessResponse, Box<dyn Error>>
    {
        let packages = if packages.is_empty() {
            None
        } else {
            Some(packages)
        };

        let payload = PackageRebuildPayload {
            packages,
            force: Some(force)
        };

        let response: SuccessResponse = self.client
            .post(format!("{}/api/packages/rebuild", self.host))
            .json(&payload)
            .send()?
            .json()?;

        Ok(response)
    }

    pub fn get_logs(&self, id: i32) -> Result<String>
    {
        let response: String = self.client
            .get(format!("{}/api/packages/{}/logs", self.host, id))
            .send()?
            .text()?;

        Ok(response)
    }

    pub fn get_workers(&self) -> Result<Vec<WorkerResponse>> {
        let response: Vec<WorkerResponse> = self.client.get(format!("{}/api/workers", self.host))
            .send()?
            .json()?;

        Ok(response)
    }

    pub fn delete_worker(&self, id: usize) -> Result<SuccessResponse> {
        Ok(
            self.client
                .delete(format!("{}/api/workers/{}", self.host, id))
                .send()?
                .json()?
        )
    }

    pub fn get_patches(&self, package_id: i32) -> Result<Vec<PackagePatchResponse>>
    {
        Ok(
            self.client
                .get(format!("{}/api/packages/{}/patches", self.host, package_id))
                .send()?
                .json()?
        )
    }

    pub fn create_patch(&self, package_id: i32, payload: CreatePackagePatchPayload) -> Result<PackagePatchResponse>
    {
        Ok(
            self.client
                .post(format!("{}/api/packages/{}/patches", self.host, package_id))
                .json(&payload)
                .send()?
                .json()?
        )
    }

    pub fn delete_patch(&self, package_id: i32, id: i32) -> Result<SuccessResponse>
    {
        Ok(
            self.client
                .delete(format!("{}/api/packages/{}/patches/{}", self.host, package_id, id))
                .send()?
                .json()?
        )
    }

    pub fn webhook_trigger_package(&self, package_name: &String) -> Result<SuccessResponse, Box<dyn Error>>
    {
        let response: SuccessResponse = self.client.post(format!("{}/api/webhook/trigger/package_updated/{}", self.host, package_name))
            .send()?
            .json()?;

        Ok(response)
    }
}