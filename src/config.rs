use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub exchange: ExchangeConfig,
    pub trading: TradingConfig,
    pub assets: AssetConfig,
    pub state_file: String,
    pub mode: TradingMode,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExchangeConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub passphrase: Option<String>,
    pub sandbox: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradingConfig {
    pub basket_count: u32,
    pub profit_threshold_percent: Decimal,
    pub min_investment_percent: Decimal,
    pub max_investment_percent: Decimal,
    pub fear_greed_threshold: u32,
    pub buy_the_dip_percent: Decimal,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetConfig {
    pub initial_fiat_amount: Decimal,
    pub initial_crypto_amount: Decimal,
    pub fiat_symbol: String,
    pub crypto_symbol: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum TradingMode {
    Live,
    Backtest,
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
