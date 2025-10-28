# Binstra - Cryptocurrency Trading Bot

A sophisticated cryptocurrency trading bot that implements a basket-based trading strategy, designed to buy low and sell high using Fear & Greed Index and price dip detection.

## 🎯 Strategy Overview

The bot implements a **non-following strategy** with these core principles:

- **Basket Trading**: Splits capital into multiple trading baskets (configurable count)
- **Buy Signals**: Purchases when Fear & Greed Index drops below threshold OR price dips significantly
- **Sell Strategy**: Sells baskets when they reach configured profit threshold
- **Enhanced Buy-the-Dip**: Dynamically scales investment amounts based on dip magnitude
- **Single Asset Focus**: Currently optimized for BTC trading

## 🏗️ Architecture

### Core Components

- **Exchange Interface**: Trait-based design for easy exchange integration
- **Basket Management**: Each basket represents one trade with buy price, quantity, and target profit
- **State Management**: Stateless bot with JSON state persistence
- **Backtesting Framework**: Comprehensive historical testing with real market data

### Supported Modes

- **Backtesting Mode**: Uses historical data for strategy validation (primary focus)
- **Live Trading Mode**: Framework ready (OKX integration not fully implemented)

## 📊 Performance Results

### Optimized Configuration Backtesting Results

**Configuration:**
- Basket Count: 75
- Profit Threshold: 3.0%
- Investment Range: 8.0% - 20.0%
- Fear & Greed Threshold: 30
- Buy-the-Dip Threshold: 5.0%

**Performance Summary:**

| Period | Return | Trades | Win Rate | Max Drawdown |
|--------|--------|--------|----------|--------------|
| **365 days** | **+71.79%** | 1,075 | 100% | 21.06% |
| **180 days** | **+18.55%** | 261 | 100% | 11.72% |
| **90 days** | **+4.59%** | 136 | 100% | 9.98% |

*Backtesting period: October 2024 - October 2025*

### Key Performance Insights

- ✅ **100% Win Rate**: All trades profitable due to strategy design
- 📈 **Strong Long-term Performance**: +71.79% annual return
- 📉 **Controlled Risk**: Maximum drawdown under 22%
- 🎯 **Consistent Execution**: 1,000+ successful trades over 365 days

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+
- Python 3.8+ (for data fetching scripts)

### Installation

```bash
git clone https://github.com/jozef-pridavok/binstra
cd binstra
cargo build --release
```

### Running Backtests

```bash
# Download historical data (365 days)
python3 backtest-scripts/fetch_historical_data_binance.py --days 365

# Run backtest with optimal configuration
cargo run --release -- -c config_optimal.toml backtest --days 365

# Run backtest with custom configuration
cargo run --release -- -c optimize/config_oxy.toml backtest --days 90
```

## ⚙️ Configuration

The bot uses TOML configuration files. Example configuration:

```toml
# config_optimal.toml - Optimized configuration
state_file = "bot_state_optimal.json"
mode = "Backtest"

[exchange]
name = "okx"
sandbox = true

[trading]
basket_count = 75
profit_threshold_percent = 3.0
min_investment_percent = 8.0
max_investment_percent = 20.0
fear_greed_threshold = 30
buy_the_dip_percent = 5.0

[assets]
initial_fiat_amount = 10000.0
initial_crypto_amount = 0.0
fiat_symbol = "USDT"
crypto_symbol = "BTC"
```

### Key Parameters

- `basket_count`: Number of trading baskets (recommended: 50-100)
- `profit_threshold_percent`: Profit target for selling (recommended: 3-5%)
- `min/max_investment_percent`: Investment range per basket
- `fear_greed_threshold`: Buy trigger threshold (lower = more aggressive)
- `buy_the_dip_percent`: Price dip detection threshold

## 🧪 Backtesting Features

### Data Sources

- **Price Data**: Binance API (reliable 1-hour candles)
- **Fear & Greed Index**: Real historical sentiment data
- **Date Range**: Supports 7, 30, 90, 180, and 365-day backtests

### Enhanced Features

- **Buy-the-Dip Scaling**: Investment amount scales with dip magnitude
- **Historical Accuracy**: Proper time simulation with real market conditions
- **Multiple Timeframes**: Test different market periods
- **Parameter Optimization**: Automated parameter tuning

## 🔬 Technical Details

### Enhanced Buy-the-Dip Logic

The bot implements dynamic investment scaling based on price drop magnitude:

- **At threshold dip (5%)**: Uses `min_investment_percent` (8%)
- **At maximum dip (100%)**: Uses `max_investment_percent` (20%)
- **Linear interpolation**: Scales investment between these bounds

### State Management

- **Stateless Design**: Bot state persisted in JSON files
- **Basket Tracking**: Individual trade tracking with profit calculations
- **Portfolio Monitoring**: Real-time portfolio value and statistics

## ⚠️ Important Notes

- **Backtesting Focus**: This project is primarily designed for strategy validation
- **OKX Integration**: Exchange integration framework exists but not fully implemented
- **Risk Warning**: Past performance does not guarantee future results
- **Educational Purpose**: Use for learning and strategy development

## 🛠️ Development

### Adding New Exchanges

The project uses a trait-based architecture for easy exchange integration:

```rust
pub trait ExchangeClient: Send + Sync {
    async fn get_prices(&self, symbols: &[String]) -> anyhow::Result<Vec<Price>>;
    async fn buy(&self, symbol: &str, amount: Decimal) -> anyhow::Result<OrderResult>;
    async fn sell(&self, symbol: &str, quantity: Decimal) -> anyhow::Result<OrderResult>;
}
```

## 📈 Future Enhancements

Potential improvements discussed:

1. **Adaptive Thresholds**: Dynamic buy-the-dip based on market volatility
2. **Multi-level Baskets**: Dollar-cost averaging down strategy
3. **Additional Indicators**: BTC dominance, RSI, volume analysis
4. **Dynamic Profit Targets**: Trend-based profit thresholds
5. **Time-based Scaling**: Profit targets that decrease over holding time

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

---

**Disclaimer**: This software is for educational and research purposes. Cryptocurrency trading involves substantial risk of loss. Always conduct your own research and consider consulting with a financial advisor before making investment decisions.