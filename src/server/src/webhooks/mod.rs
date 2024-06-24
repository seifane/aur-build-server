mod payloads;

use std::sync::Arc;
use log::{error, info};
use reqwest::Client;
use tokio::sync::RwLock;
use common::http::responses::PackageResponse;
use crate::models::config::Config;
use crate::webhooks::payloads::{WebhookPayload};


pub struct WebhookManager {
    config: Arc<RwLock<Config>>,
    client: Client
}

impl WebhookManager {
    pub fn from_config(config: Arc<RwLock<Config>>) -> Self
    {
        WebhookManager {
            config,
            client: Client::new()
        }
    }

    pub async fn trigger_webhook_package_updated(&self, package: PackageResponse) {
        if let Some(webhooks) = self.config.read().await.webhooks.as_ref() {
            for endpoint in webhooks.iter() {
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
}

