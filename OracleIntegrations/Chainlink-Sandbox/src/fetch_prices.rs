use crate::{Asset, PriceData, OracleError, OracleAdapter};
use near_sdk::json_types::U128;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Price fetcher for tokens and pools
pub struct PriceFetcher {
    oracle: Box<dyn OracleAdapter>,
    max_age: u64,  // Maximum age of data in seconds
}

impl PriceFetcher {
    pub fn new(oracle: Box<dyn OracleAdapter>, max_age: u64) -> Self {
        Self { oracle, max_age }
    }

    /// Validate data freshness
    fn validate_timestamp(&self, timestamp: u64) -> Result<(), OracleError> {
        let current = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if current - timestamp > self.max_age {
            return Err(OracleError::StaleData {
                current,
                received: timestamp,
            });
        }

        Ok(())
    }

    /// Get current prices for multiple assets
    pub async fn get_prices(&self, assets: &[Asset]) -> Result<HashMap<Asset, PriceData>, OracleError> {
        let prices = self.oracle.get_prices(assets).await?;
        
        // Validate all timestamps
        for price_data in prices.values() {
            self.validate_timestamp(price_data.timestamp)?;
        }

        Ok(prices)
    }

    /// Calculate price change percentage over a period
    pub async fn calculate_price_change(
        &self,
        asset: &Asset,
        period: u64,  // Period in seconds
    ) -> Result<f64, OracleError> {
        let current_price = self.oracle.get_price(asset).await?;
        self.validate_timestamp(current_price.timestamp)?;

        // Note: In a real implementation, we would fetch historical price
        // For now, we'll simulate a 1% change
        let simulated_old_price = U128(current_price.price.0 * 99 / 100);
        let old_price_data = PriceData {
            price: simulated_old_price,
            timestamp: current_price.timestamp - period,
            source: current_price.source.clone(),
        };

        let change = (current_price.price.0 as f64 - old_price_data.price.0 as f64) 
            / old_price_data.price.0 as f64 
            * 100.0;

        Ok(change)
    }

    /// Calculate volatility score (0-100)
    pub async fn calculate_volatility(
        &self,
        asset: &Asset,
        period: u64,
        samples: u32,
    ) -> Result<u8, OracleError> {
        // Note: In a real implementation, we would fetch historical prices
        // For now, we'll simulate some price changes
        let mut changes = Vec::new();
        let current_price = self.oracle.get_price(asset).await?;
        self.validate_timestamp(current_price.timestamp)?;

        let base_price = current_price.price.0;
        for i in 0..samples {
            // Simulate price changes with some randomness
            let change = (i as f64 * 0.01) - 0.005;  // -0.5% to +0.5%
            changes.push(change);
        }

        // Calculate volatility as standard deviation of changes
        let mean = changes.iter().sum::<f64>() / changes.len() as f64;
        let variance = changes.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / changes.len() as f64;
        let std_dev = variance.sqrt();

        // Convert to 0-100 scale (assuming max volatility of 10% standard deviation)
        let volatility = ((std_dev * 1000.0).min(100.0)) as u8;
        Ok(volatility)
    }

    /// Check if price movement exceeds threshold
    pub async fn check_price_movement(
        &self,
        asset: &Asset,
        threshold_percent: f64,
        period: u64,
    ) -> Result<bool, OracleError> {
        let change = self.calculate_price_change(asset, period).await?;
        Ok(change.abs() >= threshold_percent)
    }

    /// Get price impact estimation for a trade
    pub async fn estimate_price_impact(
        &self,
        asset: &Asset,
        amount: U128,
    ) -> Result<f64, OracleError> {
        let current_price = self.oracle.get_price(asset).await?;
        self.validate_timestamp(current_price.timestamp)?;

        // Note: This is a simplified model. In reality, we would:
        // 1. Fetch pool liquidity data
        // 2. Use AMM curve calculations
        // 3. Consider slippage and depth
        
        // Simplified impact calculation (square root model)
        let impact = (amount.0 as f64 / current_price.price.0 as f64).sqrt() * 0.01;
        Ok(impact.min(1.0) * 100.0)  // Return as percentage, max 100%
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockOracle;
    use tokio_test::block_on;

    #[test]
    fn test_price_fetcher() {
        block_on(async {
            let mock_oracle = MockOracle::new();
            let fetcher = PriceFetcher::new(Box::new(mock_oracle), 3600);

            let eth = Asset::Token("ETH".to_string());
            let usdc = Asset::Token("USDC".to_string());

            let eth_price = PriceData {
                price: U128(1_500_000_000_000),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                source: "mock".to_string(),
            };

            let usdc_price = PriceData {
                price: U128(1_000_000),  // $1
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                source: "mock".to_string(),
            };

            // Set up test data
            mock_oracle.set_price(eth.clone(), eth_price.clone()).await;
            mock_oracle.set_price(usdc.clone(), usdc_price.clone()).await;

            // Test price fetching
            let prices = fetcher.get_prices(&[eth.clone(), usdc.clone()]).await.unwrap();
            assert_eq!(prices.len(), 2);
            assert_eq!(prices.get(&eth).unwrap().price, eth_price.price);
            assert_eq!(prices.get(&usdc).unwrap().price, usdc_price.price);

            // Test price impact estimation
            let impact = fetcher.estimate_price_impact(&eth, U128(1_000_000_000_000)).await.unwrap();
            assert!(impact > 0.0 && impact <= 100.0);

            // Test volatility calculation
            let volatility = fetcher.calculate_volatility(&eth, 86400, 10).await.unwrap();
            assert!(volatility >= 0 && volatility <= 100);
        });
    }
} 