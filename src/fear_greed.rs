use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearGreedIndex {
    pub value: u32,
    pub classification: String,
    pub timestamp: DateTime<Utc>,
}

pub struct FearGreedClient {
    client: reqwest::Client,
}

impl FearGreedClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_current_index(&self) -> anyhow::Result<FearGreedIndex> {
        let url = "https://api.alternative.me/fng/";
        let response: serde_json::Value = self.client.get(url).send().await?.json().await?;

        if let Some(data) = response["data"].as_array().and_then(|arr| arr.first()) {
            let value = data["value"]
                .as_str()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(50);

            let classification = data["value_classification"]
                .as_str()
                .unwrap_or("Neutral")
                .to_string();

            Ok(FearGreedIndex {
                value,
                classification,
                timestamp: Utc::now(),
            })
        } else {
            Err(anyhow::anyhow!(
                "Failed to parse Fear & Greed index response"
            ))
        }
    }
}

impl Default for FearGreedClient {
    fn default() -> Self {
        Self::new()
    }
}
