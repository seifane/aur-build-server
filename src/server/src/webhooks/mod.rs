mod payloads;

use anyhow::Result;
use std::sync::Arc;
use log::{error, info, warn};
use reqwest::{Certificate, Client};
use tokio::sync::RwLock;
use common::http::responses::PackageResponse;
use crate::models::config::Config;
use crate::webhooks::payloads::{WebhookPayload};

pub struct WebhookManager {
    config: Arc<RwLock<Config>>,
    client: Client
}

impl WebhookManager {
    pub async fn from_config(config: Arc<RwLock<Config>>) -> Result<Self>
    {
        let mut client = Client::builder();

        if !config.read().await.webhook_verify_ssl {
            warn!("Accepting any certificate for webhooks");
            client = client.danger_accept_invalid_certs(true)
        }

        if let Some(path) = &config.read().await.webhook_certificate {
            warn!("Adding new root certificate for webhooks {}", path.display());
            client = client.add_root_certificate(Certificate::from_pem(tokio::fs::read_to_string(path).await?.as_bytes())?);
        }

        Ok(WebhookManager {
            config,
            client: client.build()?
        })
    }

    pub async fn trigger_webhook_package_updated(&self, package: PackageResponse) {
        for endpoint in self.config.read().await.webhooks.iter() {
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

