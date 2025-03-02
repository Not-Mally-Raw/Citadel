use crate::{Asset, Protocol, ApyData, OracleError, OracleAdapter};
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// APY fetcher for different protocols
pub struct ApyFetcher {
    oracle: Box<dyn OracleAdapter>,
    max_age: u64,  // Maximum age of data in seconds
}

impl ApyFetcher {
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

    /// Get APY data for multiple assets across different protocols
    pub async fn get_multi_protocol_apys(
        &self,
        assets: &[Asset],
        protocols: &[Protocol],
    ) -> Result<HashMap<(Asset, Protocol), ApyData>, OracleError> {
        let mut results = HashMap::new();

        for protocol in protocols {
            let apys = self.oracle.get_apys(assets, protocol).await?;
            for (asset, apy_data) in apys {
                self.validate_timestamp(apy_data.timestamp)?;
                results.insert((asset, protocol.clone()), apy_data);
            }
        }

        Ok(results)
    }

    /// Find the best APY for a given asset across all supported protocols
    pub async fn find_best_apy(
        &self,
        asset: &Asset,
        protocols: &[Protocol],
    ) -> Result<Option<(Protocol, ApyData)>, OracleError> {
        let mut best: Option<(Protocol, ApyData)> = None;

        for protocol in protocols {
            if let Ok(apy_data) = self.oracle.get_apy(asset, protocol).await {
                self.validate_timestamp(apy_data.timestamp)?;

                match &best {
                    None => best = Some((protocol.clone(), apy_data)),
                    Some((_, current_best)) if apy_data.apy > current_best.apy => {
                        best = Some((protocol.clone(), apy_data))
                    }
                    _ => {}
                }
            }
        }

        Ok(best)
    }

    /// Get APY history for an asset in a specific protocol
    pub async fn get_apy_history(
        &self,
        asset: &Asset,
        protocol: &Protocol,
        _period: u64, // Period in seconds
    ) -> Result<Vec<ApyData>, OracleError> {
        // Note: This is a placeholder. In a real implementation,
        // we would fetch historical data from an API or database
        let current = self.oracle.get_apy(asset, protocol).await?;
        Ok(vec![current])
    }

    /// Calculate volatility score for APY (0-100)
    pub async fn calculate_apy_volatility(
        &self,
        asset: &Asset,
        protocol: &Protocol,
        period: u64,
    ) -> Result<u8, OracleError> {
        let history = self.get_apy_history(asset, protocol, period).await?;
        
        if history.len() < 2 {
            return Ok(0);
        }

        // Calculate standard deviation of APY changes
        let changes: Vec<f64> = history.windows(2)
            .map(|w| (w[1].apy - w[0].apy).abs())
            .collect();

        let mean = changes.iter().sum::<f64>() / changes.len() as f64;
        let variance = changes.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / changes.len() as f64;
        let std_dev = variance.sqrt();

        // Convert to 0-100 scale (assuming max volatility of 100% APY change)
        let volatility = (std_dev * 100.0).min(100.0) as u8;
        Ok(volatility)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockOracle;
    use tokio_test::block_on;

    #[test]
    fn test_apy_fetcher() {
        block_on(async {
            let mock_oracle = MockOracle::new();
            let fetcher = ApyFetcher::new(Box::new(mock_oracle), 3600);

            let eth = Asset::Token("ETH".to_string());
            let aave = Protocol::Aave;
            let compound = Protocol::Compound;

            let aave_apy = ApyData {
                apy: 0.05,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                protocol: Protocol::Aave,
                risk_score: 2,
            };

            let compound_apy = ApyData {
                apy: 0.06,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                protocol: Protocol::Compound,
                risk_score: 3,
            };

            // Set up test data
            mock_oracle.set_apy(eth.clone(), aave.clone(), aave_apy.clone()).await;
            mock_oracle.set_apy(eth.clone(), compound.clone(), compound_apy.clone()).await;

            // Test finding best APY
            let best = fetcher.find_best_apy(&eth, &[aave.clone(), compound.clone()]).await.unwrap();
            assert!(best.is_some());
            let (best_protocol, best_apy) = best.unwrap();
            assert!(matches!(best_protocol, Protocol::Compound));
            assert_eq!(best_apy.apy, 0.06);

            // Test multi-protocol fetch
            let multi = fetcher.get_multi_protocol_apys(
                &[eth.clone()],
                &[aave.clone(), compound.clone()]
            ).await.unwrap();
            assert_eq!(multi.len(), 2);
            assert!(multi.contains_key(&(eth.clone(), aave.clone())));
            assert!(multi.contains_key(&(eth.clone(), compound.clone())));
        });
    }
} 