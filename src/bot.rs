use crate::{
    config::Config,
    exchange::{ExchangeClient, Price},
    basket::Basket,
    state::BotState,
    fear_greed::{FearGreedClient, FearGreedIndex},
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TradingBot {
    config: Config,
    exchange: Arc<Mutex<dyn ExchangeClient>>,
    fear_greed_client: FearGreedClient,
    state: BotState,
}

impl TradingBot {
    pub fn new(
        config: Config,
        exchange: Arc<Mutex<dyn ExchangeClient>>,
        state: BotState,
    ) -> Self {
        Self {
            config,
            exchange,
            fear_greed_client: FearGreedClient::new(),
            state,
        }
    }

    pub async fn run_cycle(&mut self) -> anyhow::Result<()> {
        self.run_cycle_with_time(None).await
    }

    pub async fn run_cycle_with_time(&mut self, simulation_time: Option<DateTime<Utc>>) -> anyhow::Result<()> {
        self.run_cycle_with_options(simulation_time, None).await
    }

    pub async fn run_cycle_with_options(&mut self, simulation_time: Option<DateTime<Utc>>, fear_greed_override: Option<FearGreedIndex>) -> anyhow::Result<()> {
        println!("Starting trading cycle at {}", chrono::Utc::now());

        // Get current prices
        let current_prices = self.get_current_prices().await?;
        let price_map = self.prices_to_map(&current_prices);

        // Update recent highs for price tracking
        for (symbol, &price) in &price_map {
            self.state.update_recent_high(symbol, price);
        }

        // Get Fear & Greed index (use override if provided, otherwise try API)
        let fear_greed_index = if let Some(override_index) = fear_greed_override {
            override_index
        } else {
            match self.fear_greed_client.get_current_index().await {
                Ok(index) => index,
                Err(_) => {
                    // Fallback for testing/backtesting when API is not available
                    crate::fear_greed::FearGreedIndex {
                        value: 35, // Default fear value
                        classification: "Fear".to_string(),
                        timestamp: chrono::Utc::now(),
                    }
                }
            }
        };
        println!("Fear & Greed Index: {} ({})", fear_greed_index.value, fear_greed_index.classification);

        // Check for sell opportunities
        self.check_sell_opportunities(&price_map).await?;

        // Check for buy opportunities
        self.check_buy_opportunities(&fear_greed_index, &price_map, simulation_time).await?;

        // Save state
        self.state.save_to_file(&self.config.state_file)?;

        println!("Trading cycle completed");
        self.print_statistics(&price_map, simulation_time);

        Ok(())
    }

    async fn get_current_prices(&self) -> anyhow::Result<Vec<Price>> {
        let exchange = self.exchange.lock().await;
        exchange.get_prices(&[self.config.assets.crypto_symbol.clone()]).await
    }

    fn prices_to_map(&self, prices: &[Price]) -> HashMap<String, Decimal> {
        prices.iter()
            .map(|p| (p.symbol.clone(), p.price))
            .collect()
    }

    async fn check_sell_opportunities(&mut self, current_prices: &HashMap<String, Decimal>) -> anyhow::Result<()> {
        let mut baskets_to_close = Vec::new();

        for basket in &self.state.active_baskets {
            if let Some(&current_price) = current_prices.get(&basket.asset) {
                if basket.should_sell(current_price) {
                    println!(
                        "Selling basket {} for {} at price {} (bought at {})",
                        basket.id, basket.asset, current_price, basket.buy_price
                    );

                    // Execute sell order
                    let exchange = self.exchange.lock().await;
                    let order_result = exchange.sell(&basket.asset, basket.quantity).await?;
                    
                    println!("Sell order executed: {:?}", order_result);
                    baskets_to_close.push((basket.id.clone(), current_price));
                }
            }
        }

        // Close sold baskets
        for (basket_id, sell_price) in baskets_to_close {
            self.state.close_basket(&basket_id, sell_price)?;
        }

        Ok(())
    }

    async fn check_buy_opportunities(
        &mut self,
        fear_greed_index: &FearGreedIndex,
        current_prices: &HashMap<String, Decimal>,
        simulation_time: Option<DateTime<Utc>>,
    ) -> anyhow::Result<()> {
        // Check buy signals: Fear & Greed OR price dip
        let crypto_symbol = &self.config.assets.crypto_symbol;
        let fear_greed_signal = fear_greed_index.value <= self.config.trading.fear_greed_threshold;
        let (dip_signal, dip_percent) = if let Some(&current_price) = current_prices.get(crypto_symbol) {
            let is_dip = self.state.is_price_dip(crypto_symbol, current_price, self.config.trading.buy_the_dip_percent);
            let actual_dip_percent = if is_dip {
                self.state.get_dip_percentage(crypto_symbol, current_price)
            } else {
                Decimal::ZERO
            };
            (is_dip, actual_dip_percent)
        } else {
            (false, Decimal::ZERO)
        };

        if !fear_greed_signal && !dip_signal {
            println!("No buy signals: F&G: {} (threshold: {}), Dip signal: {} ({}% threshold)",
                fear_greed_index.value, self.config.trading.fear_greed_threshold, 
                dip_signal, self.config.trading.buy_the_dip_percent);
            return Ok(());
        }

        if fear_greed_signal {
            println!("Fear & Greed buy signal triggered: {} <= {}", 
                fear_greed_index.value, self.config.trading.fear_greed_threshold);
        }
        if dip_signal {
            println!("Buy the dip signal triggered: price dropped {:.2}% from recent high (threshold: {}%)", 
                dip_percent, self.config.trading.buy_the_dip_percent);
        }

        // Check if we have available basket slots
        if self.state.active_baskets.len() >= self.config.trading.basket_count as usize {
            println!("All basket slots are occupied ({}/{})",
                self.state.active_baskets.len(), self.config.trading.basket_count);
            return Ok(());
        }

        // Calculate investment amount based on signals
        let investment_percent = if dip_signal {
            // For dip signal: scale investment based on dip magnitude
            self.calculate_dip_investment_percent(dip_percent)
        } else {
            // For Fear & Greed signal: use existing logic
            self.calculate_investment_percent(fear_greed_index.value)
        };
        let investment_amount = self.state.fiat_balance * investment_percent / Decimal::from(100);

        if investment_amount <= Decimal::ZERO {
            println!("No available capital for new basket");
            return Ok(());
        }

        // Check if we can buy the crypto (only one asset traded)
        let crypto_symbol = &self.config.assets.crypto_symbol;
        
        if let Some(&current_price) = current_prices.get(crypto_symbol) {
            println!("Creating new basket for {} with investment amount {}",
                crypto_symbol, investment_amount);

            // Execute buy order
            let exchange = self.exchange.lock().await;
            let order_result = exchange.buy(crypto_symbol, investment_amount).await?;

            println!("Buy order executed: {:?}", order_result);

            // Create new basket
            let current_time = simulation_time.unwrap_or_else(|| chrono::Utc::now());
            let basket = Basket::new_with_time(
                crypto_symbol.to_string(),
                order_result.quantity,
                order_result.price,
                self.config.trading.profit_threshold_percent,
                current_time,
            );

            self.state.add_basket(basket);
        }

        Ok(())
    }

    fn calculate_investment_percent(&self, fear_greed_value: u32) -> Decimal {
        // Lower fear & greed index = higher investment
        // Scale between min and max investment percentages
        let range = self.config.trading.max_investment_percent - self.config.trading.min_investment_percent;
        let fear_factor = Decimal::from(100 - fear_greed_value) / Decimal::from(100);
        self.config.trading.min_investment_percent + (range * fear_factor)
    }

    fn calculate_dip_investment_percent(&self, dip_percent: Decimal) -> Decimal {
        // At threshold dip: use min_investment_percent
        // At 100% dip (theoretical): use max_investment_percent
        // Linear interpolation between them
        
        let threshold = self.config.trading.buy_the_dip_percent;
        let max_dip = Decimal::from(100); // Theoretical 100% drop
        
        // Clamp dip_percent to be at least the threshold
        let effective_dip = dip_percent.max(threshold);
        
        // Calculate scaling factor: 0 at threshold, 1 at 100% dip
        let dip_factor = (effective_dip - threshold) / (max_dip - threshold);
        
        // Scale between min and max investment percentages
        let range = self.config.trading.max_investment_percent - self.config.trading.min_investment_percent;
        let investment_percent = self.config.trading.min_investment_percent + (range * dip_factor);
        
        println!("Dip-based investment: {:.2}% dip -> {:.2}% investment (range: {:.1}%-{:.1}%)", 
            dip_percent, investment_percent, 
            self.config.trading.min_investment_percent, 
            self.config.trading.max_investment_percent);
        
        investment_percent
    }

    fn print_statistics(&self, current_prices: &HashMap<String, Decimal>, simulation_time: Option<DateTime<Utc>>) {
        let stats = self.state.get_statistics();
        let portfolio_value = self.state.get_total_portfolio_value(current_prices);

        println!("\n=== Bot Statistics ===");
        println!("Portfolio Value: {}", portfolio_value);
        println!("Fiat Balance: {}", self.state.fiat_balance);
        println!("Active Baskets: {}", stats.active_baskets_count);
        
        // Print detailed basket status
        if !self.state.active_baskets.is_empty() {
            println!("\n--- Active Baskets Detail ---");
            for (i, basket) in self.state.active_baskets.iter().enumerate() {
                if let Some(&current_price) = current_prices.get(&basket.asset) {
                    let current_value = basket.get_current_value(current_price);
                    let invested_amount = basket.get_invested_amount();
                    let profit = basket.get_profit(current_price);
                    let profit_percent = basket.get_profit_percent(current_price);
                    let target_price = basket.buy_price * (Decimal::from(100) + basket.target_profit_percent) / Decimal::from(100);
                    let current_time = simulation_time.unwrap_or_else(|| chrono::Utc::now());
                    let days_held = (current_time - basket.buy_timestamp).num_days();
                    
                    println!("Basket {}: {} @ ${:.2}", i + 1, basket.asset, basket.buy_price);
                    println!("  Quantity: {:.6} | Target: ${:.2} ({:.1}%)", 
                        basket.quantity, target_price, basket.target_profit_percent);
                    println!("  Current: ${:.2} | Value: ${:.2} | P&L: ${:.2} ({:.2}%)", 
                        current_price, current_value, profit, profit_percent);
                    println!("  Invested: ${:.2} | Days held: {}", invested_amount, days_held);
                } else {
                    println!("Basket {}: {} (no current price available)", i + 1, basket.asset);
                }
            }
            println!("-----------------------------");
        }
        
        println!("Total Trades: {}", stats.total_trades);
        println!("Profitable Trades: {}", stats.profitable_trades);
        println!("Win Rate: {:.2}%", stats.win_rate);
        println!("Total Profit: {}", stats.total_profit);
        println!("Average Profit %: {:.2}%", stats.average_profit_percent);
        println!("======================\n");
    }

    pub fn get_state(&self) -> &BotState {
        &self.state
    }

    pub fn get_state_mut(&mut self) -> &mut BotState {
        &mut self.state
    }
}