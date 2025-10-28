use crate::exchange::{ExchangeClient, Price, OrderResult, OrderSide};
use async_trait::async_trait;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalData {
    pub timestamp: DateTime<Utc>,
    pub prices: HashMap<String, Decimal>,
}

#[derive(Clone)]
pub struct MockClient {
    historical_data: Vec<HistoricalData>,
    current_index: Arc<Mutex<usize>>,
    balances: Arc<Mutex<HashMap<String, Decimal>>>,
}

impl MockClient {
    pub fn new(historical_data: Vec<HistoricalData>, initial_balances: HashMap<String, Decimal>) -> Self {
        Self {
            historical_data,
            current_index: Arc::new(Mutex::new(0)),
            balances: Arc::new(Mutex::new(initial_balances)),
        }
    }

    pub fn advance_time(&self) {
        let mut index = self.current_index.lock().unwrap();
        if *index < self.historical_data.len() - 1 {
            *index += 1;
        }
    }

    pub fn get_current_timestamp(&self) -> DateTime<Utc> {
        let index = *self.current_index.lock().unwrap();
        self.historical_data
            .get(index)
            .map(|data| data.timestamp)
            .unwrap_or_else(Utc::now)
    }

    pub fn set_current_index(&self, index: usize) {
        let mut current_index = self.current_index.lock().unwrap();
        *current_index = index.min(self.historical_data.len().saturating_sub(1));
    }

    pub fn load_historical_data(file_path: &str) -> anyhow::Result<Vec<HistoricalData>> {
        let content = std::fs::read_to_string(file_path)?;
        let data: Vec<HistoricalData> = serde_json::from_str(&content)?;
        Ok(data)
    }
}

#[async_trait]
impl ExchangeClient for MockClient {
    async fn get_prices(&self, symbols: &[String]) -> anyhow::Result<Vec<Price>> {
        let index = *self.current_index.lock().unwrap();
        let current_data = self.historical_data
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("No historical data available"))?;


        let mut prices = Vec::new();
        for symbol in symbols {
            if let Some(&price) = current_data.prices.get(symbol) {
                prices.push(Price {
                    symbol: symbol.clone(),
                    price,
                    timestamp: current_data.timestamp,
                });
            }
        }
        Ok(prices)
    }

    async fn buy(&self, symbol: &str, amount: Decimal) -> anyhow::Result<OrderResult> {
        let prices = self.get_prices(&[symbol.to_string()]).await?;
        let price = prices.first()
            .ok_or_else(|| anyhow::anyhow!("Price not found for {}", symbol))?
            .price;

        let quantity = amount / price;
        let fee = amount * Decimal::new(1, 3); // 0.1% fee

        // Update balances
        {
            let mut balances = self.balances.lock().unwrap();
            let fiat_balance = balances.entry("USDT".to_string()).or_insert(Decimal::ZERO);
            *fiat_balance -= amount + fee;

            let crypto_balance = balances.entry(symbol.to_string()).or_insert(Decimal::ZERO);
            *crypto_balance += quantity;
        }

        Ok(OrderResult {
            order_id: format!("mock_buy_{}", chrono::Utc::now().timestamp()),
            symbol: symbol.to_string(),
            side: OrderSide::Buy,
            quantity,
            price,
            fee,
            timestamp: self.get_current_timestamp(),
        })
    }

    async fn sell(&self, symbol: &str, quantity: Decimal) -> anyhow::Result<OrderResult> {
        let prices = self.get_prices(&[symbol.to_string()]).await?;
        let price = prices.first()
            .ok_or_else(|| anyhow::anyhow!("Price not found for {}", symbol))?
            .price;

        let amount = quantity * price;
        let fee = amount * Decimal::new(1, 3); // 0.1% fee

        // Update balances
        {
            let mut balances = self.balances.lock().unwrap();
            let crypto_balance = balances.entry(symbol.to_string()).or_insert(Decimal::ZERO);
            *crypto_balance -= quantity;

            let fiat_balance = balances.entry("USDT".to_string()).or_insert(Decimal::ZERO);
            *fiat_balance += amount - fee;
        }

        Ok(OrderResult {
            order_id: format!("mock_sell_{}", chrono::Utc::now().timestamp()),
            symbol: symbol.to_string(),
            side: OrderSide::Sell,
            quantity,
            price,
            fee,
            timestamp: self.get_current_timestamp(),
        })
    }

    async fn get_balance(&self, asset: &str) -> anyhow::Result<Decimal> {
        let balances = self.balances.lock().unwrap();
        Ok(balances.get(asset).copied().unwrap_or(Decimal::ZERO))
    }
}