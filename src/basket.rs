use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Basket {
    pub id: String,
    pub asset: String,
    pub quantity: Decimal,
    pub buy_price: Decimal,
    pub buy_timestamp: DateTime<Utc>,
    pub target_profit_percent: Decimal,
}

impl Basket {
    pub fn new_with_time(
        asset: String,
        quantity: Decimal,
        buy_price: Decimal,
        target_profit_percent: Decimal,
        buy_timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: format!("basket_{}_{}", asset, buy_timestamp.timestamp()),
            asset,
            quantity,
            buy_price,
            buy_timestamp,
            target_profit_percent,
        }
    }

    pub fn should_sell(&self, current_price: Decimal) -> bool {
        let profit_percent = (current_price - self.buy_price) / self.buy_price * Decimal::from(100);
        profit_percent >= self.target_profit_percent
    }

    pub fn get_current_value(&self, current_price: Decimal) -> Decimal {
        self.quantity * current_price
    }

    pub fn get_invested_amount(&self) -> Decimal {
        self.quantity * self.buy_price
    }

    pub fn get_profit(&self, current_price: Decimal) -> Decimal {
        self.get_current_value(current_price) - self.get_invested_amount()
    }

    pub fn get_profit_percent(&self, current_price: Decimal) -> Decimal {
        (current_price - self.buy_price) / self.buy_price * Decimal::from(100)
    }
}
