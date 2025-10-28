use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub symbol: String,
    pub price: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResult {
    pub order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub price: Decimal,
    pub fee: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[async_trait]
pub trait ExchangeClient: Send + Sync {
    async fn get_prices(&self, symbols: &[String]) -> anyhow::Result<Vec<Price>>;
    async fn buy(&self, symbol: &str, amount: Decimal) -> anyhow::Result<OrderResult>;
    async fn sell(&self, symbol: &str, quantity: Decimal) -> anyhow::Result<OrderResult>;
    // async fn get_balance(&self, asset: &str) -> anyhow::Result<Decimal>;
}

pub mod mock;
pub mod okx;

// Re-export commonly used types
