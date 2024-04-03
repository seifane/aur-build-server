use std::error::Error;
use reqwest::blocking::Client;
use reqwest::header;
use reqwest::header::{HeaderMap, HeaderValue};
use common::http::payloads::PackageRebuildPayload;
use common::http::responses::{PackageResponse, SuccessResponse, WorkerResponse};

pub struct Api {
    client: Client,
    host: String,
}

impl Api {
    pub fn new(host: String, api_key: String) -> Result<Self, Box<dyn Error>>
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

    pub fn get_packages(&self) -> Result<Vec<PackageResponse>, Box<dyn Error>>
    {
        Ok(self.client.get(format!("{}/api/packages", self.host)).send()?.json()?)
    }

    pub fn rebuild_packages(&self, packages: Vec<String>) -> Result<SuccessResponse, Box<dyn Error>>
    {
        let packages = if packages.is_empty() {
            None
        } else {
            Some(packages)
        };

        let payload = PackageRebuildPayload {
            packages,
        };

        let response: SuccessResponse = self.client.post(format!("{}/api/rebuild", self.host))
            .json(&payload)
            .send()?
            .json()?;

        Ok(response)
    }

    pub fn get_logs(&self, package: &String) -> Result<String, Box<dyn Error>>
    {
        let response: String = self.client.get(format!("{}/api/logs/{}", self.host, package))
            .send()?.text()?;

        Ok(response)
    }

    pub fn get_workers(&self) -> Result<Vec<WorkerResponse>, Box<dyn Error>> {
        let response: Vec<WorkerResponse> = self.client.get(format!("{}/api/workers", self.host))
            .send()?
            .json()?;

        Ok(response)
    }
}