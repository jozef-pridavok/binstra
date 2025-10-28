use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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
        let response: serde_json::Value = self.client
            .get(url)
            .send()
            .await?
            .json()
            .await?;

        if let Some(data) = response["data"].as_array().and_then(|arr| arr.first()) {
            let value = data["value"].as_str()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(50);
            
            let classification = data["value_classification"].as_str()
                .unwrap_or("Neutral")
                .to_string();

            Ok(FearGreedIndex {
                value,
                classification,
                timestamp: Utc::now(),
            })
        } else {
            Err(anyhow::anyhow!("Failed to parse Fear & Greed index response"))
        }
    }

    pub async fn get_historical_index(&self, days: u32) -> anyhow::Result<Vec<FearGreedIndex>> {
        let url = format!("https://api.alternative.me/fng/?limit={}", days);
        let response: serde_json::Value = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        let mut indices = Vec::new();
        if let Some(data_array) = response["data"].as_array() {
            for data in data_array {
                if let (Some(value_str), Some(classification), Some(timestamp_str)) = (
                    data["value"].as_str(),
                    data["value_classification"].as_str(),
                    data["timestamp"].as_str(),
                ) {
                    if let (Ok(value), Ok(timestamp)) = (
                        value_str.parse::<u32>(),
                        timestamp_str.parse::<i64>(),
                    ) {
                        indices.push(FearGreedIndex {
                            value,
                            classification: classification.to_string(),
                            timestamp: DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now),
                        });
                    }
                }
            }
        }

        Ok(indices)
    }
}

impl Default for FearGreedClient {
    fn default() -> Self {
        Self::new()
    }
}