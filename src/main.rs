mod backtest;
mod basket;
mod bot;
mod config;
mod exchange;
mod fear_greed;
mod state;

use backtest::Backtester;
use bot::TradingBot;
use clap::{Parser, Subcommand};
use config::{Config, TradingMode};
use exchange::mock::MockClient;
use state::BotState;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(name = "binstra")]
#[command(about = "Crypto trading bot with backtesting capabilities")]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the trading bot
    Run,
    /// Run backtesting
    Backtest {
        /// Number of days to backtest (30, 90, or 180)
        #[arg(short, long)]
        days: u32,
    },
    /// Fetch historical data for backtesting
    FetchData {
        /// Number of days to fetch
        #[arg(short, long, default_value = "180")]
        days: u32,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let config = Config::from_file(&cli.config)?;

    match cli.command {
        Commands::Run => {
            run_trading_bot(config).await?;
        }
        Commands::Backtest { days } => {
            run_backtest(config, days).await?;
        }
        Commands::FetchData { days } => {
            fetch_historical_data(days).await?;
        }
    }

    Ok(())
}

async fn run_trading_bot(config: Config) -> anyhow::Result<()> {
    println!("Starting Binstra Trading Bot...");

    match config.mode {
        TradingMode::Live => {
            // TODO: Implement live trading with OKX
            println!("Live trading mode not yet implemented");
            return Ok(());
        }
        TradingMode::Backtest => {
            println!("Running in backtest mode");
        }
    }

    // Load or create bot state
    let state = if std::path::Path::new(&config.state_file).exists() {
        BotState::load_from_file(&config.state_file)?
    } else {
        BotState::new(
            config.assets.initial_fiat_amount,
            config.assets.crypto_symbol.clone(),
            config.assets.initial_crypto_amount,
        )
    };

    // Create mock client for testing
    let mut initial_balances = HashMap::new();
    initial_balances.insert(
        config.assets.fiat_symbol.clone(),
        config.assets.initial_fiat_amount,
    );
    initial_balances.insert(
        config.assets.crypto_symbol.clone(),
        config.assets.initial_crypto_amount,
    );

    let mock_client = MockClient::new(Vec::new(), initial_balances);
    let exchange = Arc::new(Mutex::new(mock_client));

    // Create and run bot
    let mut bot = TradingBot::new(config, exchange, state);
    bot.run_cycle().await?;

    Ok(())
}

async fn run_backtest(config: Config, days: u32) -> anyhow::Result<()> {
    println!("Running backtest for {days} days...");

    let mut backtester = Backtester::new(config);
    backtester.load_historical_data(days)?;

    let result = backtester.run_backtest(days).await?;
    backtester.save_result(&result)?;

    // Print results
    println!("\n=== BACKTEST RESULTS ===");
    println!(
        "Period: {} days ({} to {})",
        result.period_days,
        result.start_date.format("%Y-%m-%d"),
        result.end_date.format("%Y-%m-%d")
    );
    println!(
        "Initial Portfolio Value: ${}",
        result.initial_portfolio_value
    );
    println!("Final Portfolio Value: ${}", result.final_portfolio_value);
    println!(
        "Total Return: ${} ({:.2}%)",
        result.total_return, result.total_return_percent
    );
    println!("Total Trades: {}", result.total_trades);
    println!("Profitable Trades: {}", result.profitable_trades);
    println!("Win Rate: {:.2}%", result.win_rate);
    println!(
        "Max Drawdown: ${} ({:.2}%)",
        result.max_drawdown, result.max_drawdown_percent
    );
    println!("========================");

    Ok(())
}

async fn fetch_historical_data(days: u32) -> anyhow::Result<()> {
    println!("Fetching historical data for {days} days...");
    println!("Please run the Python script manually:");
    println!("cd backtest-scripts && python3 fetch_historical_data.py --days {days}");
    Ok(())
}
