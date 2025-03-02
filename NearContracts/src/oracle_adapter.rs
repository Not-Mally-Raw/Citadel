use near_sdk::{env, AccountId, Balance, Promise};
use serde::{Deserialize, Serialize};

const CHAINLINK_FEED_REGISTRY: &str = "feed.testnet.chainlink.near";
const UPDATE_THRESHOLD: u64 = 3600; // 1 hour in seconds
const HEARTBEAT_THRESHOLD: u64 = 86400; // 24 hours in seconds

#[derive(Serialize, Deserialize, Clone)]
pub struct PriceFeed {
    pub token: String,
    pub price: u128,
    pub decimals: u8,
    pub last_update: u64,
    pub heartbeat: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct APYFeed {
    pub protocol: String,
    pub apy: u32,
    pub tvl: Balance,
    pub last_update: u64,
}

#[derive(Serialize, Deserialize)]
pub struct LiquidityMetrics {
    pub token: String,
    pub total_liquidity: Balance,
    pub available_liquidity: Balance,
    pub utilization_rate: u32,
    pub last_update: u64,
}

pub struct OracleAdapter {
    price_feeds: Vec<PriceFeed>,
    apy_feeds: Vec<APYFeed>,
    liquidity_metrics: Vec<LiquidityMetrics>,
    last_health_check: u64,
}

impl OracleAdapter {
    pub fn new() -> Self {
        Self {
            price_feeds: Vec::new(),
            apy_feeds: Vec::new(),
            liquidity_metrics: Vec::new(),
            last_health_check: env::block_timestamp(),
        }
    }

    pub async fn fetch_price(&mut self, token: &str) -> Result<u128, String> {
        // Check cache first
        if let Some(feed) = self.price_feeds
            .iter()
            .find(|f| f.token == token)
        {
            if env::block_timestamp() - feed.last_update < UPDATE_THRESHOLD {
                return Ok(feed.price);
            }
        }

        // Fetch from Chainlink
        let price = self.fetch_chainlink_price(token).await?;
        
        // Update cache
        self.update_price_feed(token, price);
        
        Ok(price)
    }

    pub async fn fetch_apy(&mut self, protocol: &str) -> Result<u32, String> {
        // Check cache
        if let Some(feed) = self.apy_feeds
            .iter()
            .find(|f| f.protocol == protocol)
        {
            if env::block_timestamp() - feed.last_update < UPDATE_THRESHOLD {
                return Ok(feed.apy);
            }
        }

        // Fetch from protocol
        let (apy, tvl) = self.fetch_protocol_metrics(protocol).await?;
        
        // Update cache
        self.update_apy_feed(protocol, apy, tvl);
        
        Ok(apy)
    }

    pub async fn fetch_liquidity_metrics(
        &mut self,
        token: &str
    ) -> Result<LiquidityMetrics, String> {
        // Check cache
        if let Some(metrics) = self.liquidity_metrics
            .iter()
            .find(|m| m.token == token)
        {
            if env::block_timestamp() - metrics.last_update < UPDATE_THRESHOLD {
                return Ok(metrics.clone());
            }
        }

        // Fetch from protocols
        let metrics = self.fetch_protocol_liquidity(token).await?;
        
        // Update cache
        self.update_liquidity_metrics(metrics.clone());
        
        Ok(metrics)
    }

    pub fn check_oracle_health(&mut self) -> bool {
        let current_time = env::block_timestamp();
        
        // Check price feed health
        for feed in &self.price_feeds {
            if current_time - feed.last_update > feed.heartbeat {
                return false;
            }
        }

        // Check APY feed health
        for feed in &self.apy_feeds {
            if current_time - feed.last_update > HEARTBEAT_THRESHOLD {
                return false;
            }
        }

        self.last_health_check = current_time;
        true
    }

    async fn fetch_chainlink_price(&self, token: &str) -> Result<u128, String> {
        // This would call the Chainlink feed registry
        // For now, return mock data
        Ok(1_000_000) // $1.00 with 6 decimals
    }

    async fn fetch_protocol_metrics(&self, protocol: &str) -> Result<(u32, Balance), String> {
        // This would fetch actual protocol metrics
        // For now, return mock data
        Ok((1000, 1_000_000)) // 10% APY and $1M TVL
    }

    async fn fetch_protocol_liquidity(&self, token: &str) -> Result<LiquidityMetrics, String> {
        // This would fetch actual liquidity data
        // For now, return mock data
        Ok(LiquidityMetrics {
            token: token.to_string(),
            total_liquidity: 1_000_000,
            available_liquidity: 800_000,
            utilization_rate: 8000, // 80%
            last_update: env::block_timestamp(),
        })
    }

    fn update_price_feed(&mut self, token: &str, price: u128) {
        if let Some(feed) = self.price_feeds
            .iter_mut()
            .find(|f| f.token == token)
        {
            feed.price = price;
            feed.last_update = env::block_timestamp();
        } else {
            self.price_feeds.push(PriceFeed {
                token: token.to_string(),
                price,
                decimals: 6,
                last_update: env::block_timestamp(),
                heartbeat: HEARTBEAT_THRESHOLD,
            });
        }
    }

    fn update_apy_feed(&mut self, protocol: &str, apy: u32, tvl: Balance) {
        if let Some(feed) = self.apy_feeds
            .iter_mut()
            .find(|f| f.protocol == protocol)
        {
            feed.apy = apy;
            feed.tvl = tvl;
            feed.last_update = env::block_timestamp();
        } else {
            self.apy_feeds.push(APYFeed {
                protocol: protocol.to_string(),
                apy,
                tvl,
                last_update: env::block_timestamp(),
            });
        }
    }

    fn update_liquidity_metrics(&mut self, metrics: LiquidityMetrics) {
        if let Some(existing) = self.liquidity_metrics
            .iter_mut()
            .find(|m| m.token == metrics.token)
        {
            *existing = metrics;
        } else {
            self.liquidity_metrics.push(metrics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn setup_context() {
        let context = VMContextBuilder::new()
            .block_timestamp(1_000_000_000)
            .build();
        testing_env!(context);
    }

    #[test]
    fn test_oracle_health_check() {
        setup_context();
        let mut oracle = OracleAdapter::new();
        
        // Add some test feeds
        oracle.update_price_feed("ETH", 1_000_000);
        oracle.update_apy_feed("aave", 1000, 1_000_000);
        
        assert!(oracle.check_oracle_health());
    }

    #[test]
    fn test_price_feed_caching() {
        setup_context();
        let mut oracle = OracleAdapter::new();
        
        oracle.update_price_feed("BTC", 20_000_000_000);
        
        // Should use cached value
        assert_eq!(
            oracle.price_feeds
                .iter()
                .find(|f| f.token == "BTC")
                .unwrap()
                .price,
            20_000_000_000
        );
    }
} 