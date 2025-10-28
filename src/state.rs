use crate::basket::Basket;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotState {
    pub fiat_balance: Decimal,
    pub crypto_balances: HashMap<String, Decimal>,
    pub active_baskets: Vec<Basket>,
    pub closed_baskets: Vec<ClosedBasket>,
    pub last_update: DateTime<Utc>,
    pub total_invested: Decimal,
    pub total_profit: Decimal,
    pub recent_highs: HashMap<String, Decimal>, // Symbol -> Recent high price
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedBasket {
    pub basket: Basket,
    pub sell_price: Decimal,
    pub sell_timestamp: DateTime<Utc>,
    pub profit: Decimal,
    pub profit_percent: Decimal,
}

impl BotState {
    pub fn new(
        initial_fiat: Decimal,
        crypto_symbol: String,
        initial_crypto_amount: Decimal,
    ) -> Self {
        let mut crypto_balances = HashMap::new();
        crypto_balances.insert(crypto_symbol, initial_crypto_amount);

        Self {
            fiat_balance: initial_fiat,
            crypto_balances,
            active_baskets: Vec::new(),
            closed_baskets: Vec::new(),
            last_update: Utc::now(),
            total_invested: Decimal::ZERO,
            total_profit: Decimal::ZERO,
            recent_highs: HashMap::new(),
        }
    }

    pub fn save_to_file(&self, file_path: &str) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(file_path, json)?;
        Ok(())
    }

    pub fn load_from_file(file_path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(file_path)?;
        let state: BotState = serde_json::from_str(&content)?;
        Ok(state)
    }

    pub fn add_basket(&mut self, basket: Basket) {
        let invested_amount = basket.get_invested_amount();
        self.fiat_balance -= invested_amount;
        self.total_invested += invested_amount;
        self.active_baskets.push(basket);
        self.last_update = Utc::now();
    }

    pub fn close_basket(&mut self, basket_id: &str, sell_price: Decimal) -> anyhow::Result<()> {
        if let Some(index) = self.active_baskets.iter().position(|b| b.id == basket_id) {
            let basket = self.active_baskets.remove(index);
            let sell_amount = basket.quantity * sell_price;
            let profit = basket.get_profit(sell_price);
            let profit_percent = basket.get_profit_percent(sell_price);

            self.fiat_balance += sell_amount;
            self.total_profit += profit;

            let closed_basket = ClosedBasket {
                basket,
                sell_price,
                sell_timestamp: Utc::now(),
                profit,
                profit_percent,
            };

            self.closed_baskets.push(closed_basket);
            self.last_update = Utc::now();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Basket with id {basket_id} not found"))
        }
    }

    pub fn get_total_portfolio_value(&self, current_prices: &HashMap<String, Decimal>) -> Decimal {
        let mut total = self.fiat_balance;

        // Add value of active baskets
        for basket in &self.active_baskets {
            if let Some(&price) = current_prices.get(&basket.asset) {
                total += basket.get_current_value(price);
            }
        }

        // Add crypto balances
        for (asset, &balance) in &self.crypto_balances {
            if let Some(&price) = current_prices.get(asset) {
                total += balance * price;
            }
        }

        total
    }

    pub fn get_statistics(&self) -> BotStatistics {
        let total_trades = self.closed_baskets.len() as u32;
        let profitable_trades = self
            .closed_baskets
            .iter()
            .filter(|cb| cb.profit > Decimal::ZERO)
            .count() as u32;

        let win_rate = if total_trades > 0 {
            (profitable_trades as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };

        let average_profit_percent = if !self.closed_baskets.is_empty() {
            let sum: Decimal = self.closed_baskets.iter().map(|cb| cb.profit_percent).sum();
            sum / Decimal::from(self.closed_baskets.len())
        } else {
            Decimal::ZERO
        };

        BotStatistics {
            total_trades,
            profitable_trades,
            win_rate,
            total_profit: self.total_profit,
            average_profit_percent,
            active_baskets_count: self.active_baskets.len() as u32,
        }
    }

    pub fn update_recent_high(&mut self, symbol: &str, current_price: Decimal) {
        let recent_high = self
            .recent_highs
            .entry(symbol.to_string())
            .or_insert(current_price);
        if current_price > *recent_high {
            *recent_high = current_price;
        }
    }

    pub fn is_price_dip(
        &self,
        symbol: &str,
        current_price: Decimal,
        dip_threshold_percent: Decimal,
    ) -> bool {
        if let Some(&recent_high) = self.recent_highs.get(symbol) {
            if recent_high > Decimal::ZERO {
                let drop_percent = (recent_high - current_price) / recent_high * Decimal::from(100);
                return drop_percent >= dip_threshold_percent;
            }
        }
        false
    }

    pub fn get_dip_percentage(&self, symbol: &str, current_price: Decimal) -> Decimal {
        if let Some(&recent_high) = self.recent_highs.get(symbol) {
            if recent_high > Decimal::ZERO {
                let drop_percent = (recent_high - current_price) / recent_high * Decimal::from(100);
                return drop_percent.max(Decimal::ZERO); // Never return negative dip
            }
        }
        Decimal::ZERO
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotStatistics {
    pub total_trades: u32,
    pub profitable_trades: u32,
    pub win_rate: f64,
    pub total_profit: Decimal,
    pub average_profit_percent: Decimal,
    pub active_baskets_count: u32,
}
