use crate::{
    config::Config,
    exchange::{ExchangeClient, mock::{MockClient, HistoricalData}},
    bot::TradingBot,
    state::BotState,
    fear_greed::FearGreedIndex,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub period_days: u32,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub initial_portfolio_value: Decimal,
    pub final_portfolio_value: Decimal,
    pub total_return: Decimal,
    pub total_return_percent: Decimal,
    pub total_trades: u32,
    pub profitable_trades: u32,
    pub win_rate: f64,
    pub max_drawdown: Decimal,
    pub max_drawdown_percent: Decimal,
    pub config_used: BacktestConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub basket_count: u32,
    pub profit_threshold_percent: Decimal,
    pub min_investment_percent: Decimal,
    pub max_investment_percent: Decimal,
    pub fear_greed_threshold: u32,
}

impl From<&Config> for BacktestConfig {
    fn from(config: &Config) -> Self {
        Self {
            basket_count: config.trading.basket_count,
            profit_threshold_percent: config.trading.profit_threshold_percent,
            min_investment_percent: config.trading.min_investment_percent,
            max_investment_percent: config.trading.max_investment_percent,
            fear_greed_threshold: config.trading.fear_greed_threshold,
        }
    }
}

pub struct Backtester {
    config: Config,
    historical_data: Vec<HistoricalData>,
    fear_greed_data: Vec<FearGreedIndex>,
}

impl Backtester {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            historical_data: Vec::new(),
            fear_greed_data: Vec::new(),
        }
    }

    pub fn load_historical_data(&mut self, days: u32) -> anyhow::Result<()> {
        // Load price data for the single tradeable asset
        let mut combined_data: HashMap<DateTime<Utc>, HashMap<String, Decimal>> = HashMap::new();

        let asset = &self.config.assets.crypto_symbol;
        let file_path = format!("backtest-data/{}_prices_{}d.json", asset.to_lowercase(), days);
        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read price data for {}: {}", asset, e))?;
        
        let asset_data: Vec<serde_json::Value> = serde_json::from_str(&content)?;
        
        for item in asset_data {
            if let (Some(timestamp_str), Some(prices)) = (
                item["timestamp"].as_str(),
                item["prices"].as_object(),
            ) {
                if let Ok(timestamp) = timestamp_str.parse::<DateTime<Utc>>() {
                    let entry = combined_data.entry(timestamp).or_insert_with(HashMap::new);
                    
                    for (symbol, price_value) in prices {
                        let price_decimal = if let Some(price_num) = price_value.as_f64() {
                            Decimal::from_f64_retain(price_num).unwrap_or_default()
                        } else if let Some(price_str) = price_value.as_str() {
                            price_str.parse::<Decimal>().unwrap_or_default()
                        } else {
                            Decimal::ZERO
                        };
                        entry.insert(symbol.clone(), price_decimal);
                    }
                }
            }
        }

        // Convert to HistoricalData format
        let mut historical_data: Vec<_> = combined_data
            .into_iter()
            .map(|(timestamp, prices)| HistoricalData { timestamp, prices })
            .collect();

        historical_data.sort_by_key(|data| data.timestamp);
        self.historical_data = historical_data;

        // Load Fear & Greed data
        let fear_greed_file = format!("backtest-data/fear_greed_{}d.json", days);
        if std::path::Path::new(&fear_greed_file).exists() {
            let content = std::fs::read_to_string(&fear_greed_file)?;
            self.fear_greed_data = serde_json::from_str(&content)?;
        }

        println!("Loaded {} historical data points and {} Fear & Greed data points",
            self.historical_data.len(), self.fear_greed_data.len());

        Ok(())
    }

    pub async fn run_backtest(&self, days: u32) -> anyhow::Result<BacktestResult> {
        if self.historical_data.is_empty() {
            return Err(anyhow::anyhow!("No historical data loaded"));
        }

        let start_date = self.historical_data.first().unwrap().timestamp;
        let end_date = self.historical_data.last().unwrap().timestamp;

        // Create initial balances
        let mut initial_balances = HashMap::new();
        initial_balances.insert(self.config.assets.fiat_symbol.clone(), self.config.assets.initial_fiat_amount);
        initial_balances.insert(self.config.assets.crypto_symbol.clone(), self.config.assets.initial_crypto_amount);

        // Create mock client with historical data
        let mock_client = Arc::new(MockClient::new(self.historical_data.clone(), initial_balances.clone()));
        let exchange: Arc<Mutex<dyn ExchangeClient>> = Arc::new(Mutex::new(mock_client.as_ref().clone()));

        // Create bot state
        let bot_state = BotState::new(
            self.config.assets.initial_fiat_amount,
            self.config.assets.crypto_symbol.clone(),
            self.config.assets.initial_crypto_amount,
        );

        // Create trading bot
        let mut bot = TradingBot::new(self.config.clone(), exchange.clone(), bot_state);

        // Track portfolio values for drawdown calculation
        let mut portfolio_values = Vec::new();
        let mut max_value = Decimal::ZERO;
        let mut max_drawdown = Decimal::ZERO;

        // Get initial portfolio value
        let initial_prices: HashMap<String, Decimal> = self.historical_data[0].prices.clone();
        let initial_portfolio_value = bot.get_state().get_total_portfolio_value(&initial_prices);
        portfolio_values.push(initial_portfolio_value);
        max_value = initial_portfolio_value;

        println!("Starting backtest from {} to {}", start_date, end_date);
        println!("Initial portfolio value: {}", initial_portfolio_value);

        // Run simulation
        for (i, data_point) in self.historical_data.iter().enumerate() {
            // Advance mock client time to current data point
            mock_client.set_current_index(i);

            // Get Fear & Greed index for this timestamp
            let fear_greed_index = self.get_fear_greed_for_timestamp(data_point.timestamp);
            
            // Run bot cycle with current market data, simulation time, and Fear & Greed override
            bot.run_cycle_with_options(Some(data_point.timestamp), fear_greed_index).await?;

            // Calculate current portfolio value
            let current_portfolio_value = bot.get_state().get_total_portfolio_value(&data_point.prices);
            portfolio_values.push(current_portfolio_value);

            // Update max value and calculate drawdown
            if current_portfolio_value > max_value {
                max_value = current_portfolio_value;
            } else {
                let drawdown = max_value - current_portfolio_value;
                if drawdown > max_drawdown {
                    max_drawdown = drawdown;
                }
            }

            // Print progress
            if i % 24 == 0 { // Every day (24 hours)
                println!("Day {}: Portfolio value: {}, Active baskets: {}",
                    i / 24, current_portfolio_value, bot.get_state().active_baskets.len());
            }
        }

        // Calculate final results
        let final_prices: HashMap<String, Decimal> = self.historical_data.last().unwrap().prices.clone();
        let final_portfolio_value = bot.get_state().get_total_portfolio_value(&final_prices);
        let total_return = final_portfolio_value - initial_portfolio_value;
        let total_return_percent = if initial_portfolio_value > Decimal::ZERO {
            (total_return / initial_portfolio_value) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let max_drawdown_percent = if max_value > Decimal::ZERO {
            (max_drawdown / max_value) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let stats = bot.get_state().get_statistics();

        let result = BacktestResult {
            period_days: days,
            start_date,
            end_date,
            initial_portfolio_value,
            final_portfolio_value,
            total_return,
            total_return_percent,
            total_trades: stats.total_trades,
            profitable_trades: stats.profitable_trades,
            win_rate: stats.win_rate,
            max_drawdown,
            max_drawdown_percent,
            config_used: BacktestConfig::from(&self.config),
        };

        println!("\nBacktest completed!");
        println!("Total return: {} ({:.2}%)", total_return, total_return_percent);
        println!("Max drawdown: {} ({:.2}%)", max_drawdown, max_drawdown_percent);
        println!("Win rate: {:.2}%", stats.win_rate);

        Ok(result)
    }

    fn get_fear_greed_for_timestamp(&self, timestamp: DateTime<Utc>) -> Option<FearGreedIndex> {
        // Find the closest Fear & Greed index entry
        self.fear_greed_data
            .iter()
            .min_by_key(|fg| (fg.timestamp - timestamp).num_seconds().abs())
            .cloned()
    }


    pub fn save_result(&self, result: &BacktestResult) -> anyhow::Result<()> {
        let filename = format!("backtest-data/backtest_result_{}d.json", result.period_days);
        let json = serde_json::to_string_pretty(result)?;
        std::fs::write(&filename, json)?;
        println!("Backtest result saved to {}", filename);
        Ok(())
    }
}

