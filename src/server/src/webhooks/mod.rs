mod payloads;

use log::{error, info};
use reqwest::Client;
use common::http::responses::PackageResponse;
use crate::models::config::Config;
use crate::webhooks::payloads::{WebhookPayload};


pub struct WebhookManager {
    endpoints: Vec<String>,
    client: Client
}

impl WebhookManager {
    pub fn from_config(config: &Config) -> Self
    {
        WebhookManager {
            endpoints: match config.webhooks.as_ref() {
                None => Vec::new(),
                Some(endpoints) => endpoints.clone()
            },
            client: Client::new()
        }
    }

    pub async fn trigger_webhook_package_updated(&self, package: PackageResponse) {
        for endpoint in self.endpoints.iter() {
            let response = self.client.post(endpoint)
                .json(&WebhookPayload::PackageUpdated(package.clone()))
                .send()
                .await;

            match response {
                Ok(response) => {
                    let response_code = response.status();
                    let response_payload = response.text().await;
                    info!("Webhook {} got response {} {:?}", endpoint, response_code, response_payload);
                }
                Err(err) => {
                    error!("Failed to deliver webhook {} got error {:?}", endpoint, err);
                }
            }
        }
    }
}

