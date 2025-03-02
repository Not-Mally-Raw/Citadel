use async_trait::async_trait;
use near_sdk::json_types::U128;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Represents different types of assets we track
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Asset {
    Token(String),    // e.g., "ETH", "USDC"
    Pool(String),     // e.g., "AAVE-USDC", "Curve-3pool"
}

/// Represents different types of protocols we interact with
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Protocol {
    Aave,
    Compound,
    Curve,
    Uniswap,
    Custom(String),
}

/// Price data from oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub price: U128,
    pub timestamp: u64,
    pub source: String,
}

/// APY data from oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApyData {
    pub apy: f64,
    pub timestamp: u64,
    pub protocol: Protocol,
    pub risk_score: u8,
}

/// Liquidity data from oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityData {
    pub total_liquidity: U128,
    pub available_liquidity: U128,
    pub utilization_rate: f64,
    pub timestamp: u64,
}

/// Oracle errors
#[derive(Error, Debug)]
pub enum OracleError {
    #[error("Failed to fetch data: {0}")]
    FetchError(String),
    
    #[error("Invalid response format: {0}")]
    InvalidFormat(String),
    
    #[error("Asset not supported: {0}")]
    UnsupportedAsset(String),
    
    #[error("Protocol not supported: {0}")]
    UnsupportedProtocol(String),
    
    #[error("Data too old: current={current}, received={received}")]
    StaleData { current: u64, received: u64 },
}

/// Main oracle adapter trait
#[async_trait]
pub trait OracleAdapter {
    /// Fetch current price for an asset
    async fn get_price(&self, asset: &Asset) -> Result<PriceData, OracleError>;
    
    /// Fetch current APY for an asset in a protocol
    async fn get_apy(&self, asset: &Asset, protocol: &Protocol) -> Result<ApyData, OracleError>;
    
    /// Fetch liquidity data for an asset in a protocol
    async fn get_liquidity(&self, asset: &Asset, protocol: &Protocol) -> Result<LiquidityData, OracleError>;
    
    /// Fetch multiple prices at once
    async fn get_prices(&self, assets: &[Asset]) -> Result<HashMap<Asset, PriceData>, OracleError>;
    
    /// Fetch multiple APYs at once
    async fn get_apys(&self, assets: &[Asset], protocol: &Protocol) -> Result<HashMap<Asset, ApyData>, OracleError>;
}

/// Chainlink oracle implementation
pub struct ChainlinkOracle {
    endpoint: String,
    api_key: Option<String>,
}

impl ChainlinkOracle {
    pub fn new(endpoint: String, api_key: Option<String>) -> Self {
        Self { endpoint, api_key }
    }

    async fn make_request(&self, path: &str) -> Result<reqwest::Response, OracleError> {
        let client = reqwest::Client::new();
        let mut request = client.get(format!("{}/{}", self.endpoint, path));
        
        if let Some(key) = &self.api_key {
            request = request.header("X-API-KEY", key);
        }
        
        request
            .send()
            .await
            .map_err(|e| OracleError::FetchError(e.to_string()))
    }
}

#[async_trait]
impl OracleAdapter for ChainlinkOracle {
    async fn get_price(&self, asset: &Asset) -> Result<PriceData, OracleError> {
        let path = match asset {
            Asset::Token(symbol) => format!("price/{}", symbol),
            Asset::Pool(name) => format!("pool/price/{}", name),
        };
        
        let response = self.make_request(&path).await?;
        
        response
            .json::<PriceData>()
            .await
            .map_err(|e| OracleError::InvalidFormat(e.to_string()))
    }
    
    async fn get_apy(&self, asset: &Asset, protocol: &Protocol) -> Result<ApyData, OracleError> {
        let protocol_str = match protocol {
            Protocol::Aave => "aave",
            Protocol::Compound => "compound",
            Protocol::Curve => "curve",
            Protocol::Uniswap => "uniswap",
            Protocol::Custom(name) => name,
        };
        
        let path = match asset {
            Asset::Token(symbol) => format!("apy/{}/{}", protocol_str, symbol),
            Asset::Pool(name) => format!("pool/apy/{}/{}", protocol_str, name),
        };
        
        let response = self.make_request(&path).await?;
        
        response
            .json::<ApyData>()
            .await
            .map_err(|e| OracleError::InvalidFormat(e.to_string()))
    }
    
    async fn get_liquidity(&self, asset: &Asset, protocol: &Protocol) -> Result<LiquidityData, OracleError> {
        let protocol_str = match protocol {
            Protocol::Aave => "aave",
            Protocol::Compound => "compound",
            Protocol::Curve => "curve",
            Protocol::Uniswap => "uniswap",
            Protocol::Custom(name) => name,
        };
        
        let path = match asset {
            Asset::Token(symbol) => format!("liquidity/{}/{}", protocol_str, symbol),
            Asset::Pool(name) => format!("pool/liquidity/{}/{}", protocol_str, name),
        };
        
        let response = self.make_request(&path).await?;
        
        response
            .json::<LiquidityData>()
            .await
            .map_err(|e| OracleError::InvalidFormat(e.to_string()))
    }
    
    async fn get_prices(&self, assets: &[Asset]) -> Result<HashMap<Asset, PriceData>, OracleError> {
        let mut prices = HashMap::new();
        
        for asset in assets {
            match self.get_price(asset).await {
                Ok(price) => { prices.insert(asset.clone(), price); },
                Err(e) => return Err(e),
            }
        }
        
        Ok(prices)
    }
    
    async fn get_apys(&self, assets: &[Asset], protocol: &Protocol) -> Result<HashMap<Asset, ApyData>, OracleError> {
        let mut apys = HashMap::new();
        
        for asset in assets {
            match self.get_apy(asset, protocol).await {
                Ok(apy) => { apys.insert(asset.clone(), apy); },
                Err(e) => return Err(e),
            }
        }
        
        Ok(apys)
    }
}

/// Mock oracle for testing
#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    
    pub struct MockOracle {
        prices: Arc<RwLock<HashMap<Asset, PriceData>>>,
        apys: Arc<RwLock<HashMap<(Asset, Protocol), ApyData>>>,
        liquidity: Arc<RwLock<HashMap<(Asset, Protocol), LiquidityData>>>,
    }
    
    impl MockOracle {
        pub fn new() -> Self {
            Self {
                prices: Arc::new(RwLock::new(HashMap::new())),
                apys: Arc::new(RwLock::new(HashMap::new())),
                liquidity: Arc::new(RwLock::new(HashMap::new())),
            }
        }
        
        pub async fn set_price(&self, asset: Asset, price: PriceData) {
            self.prices.write().await.insert(asset, price);
        }
        
        pub async fn set_apy(&self, asset: Asset, protocol: Protocol, apy: ApyData) {
            self.apys.write().await.insert((asset, protocol), apy);
        }
        
        pub async fn set_liquidity(&self, asset: Asset, protocol: Protocol, data: LiquidityData) {
            self.liquidity.write().await.insert((asset, protocol), data);
        }
    }
    
    #[async_trait]
    impl OracleAdapter for MockOracle {
        async fn get_price(&self, asset: &Asset) -> Result<PriceData, OracleError> {
            self.prices
                .read()
                .await
                .get(asset)
                .cloned()
                .ok_or_else(|| OracleError::UnsupportedAsset(format!("{:?}", asset)))
        }
        
        async fn get_apy(&self, asset: &Asset, protocol: &Protocol) -> Result<ApyData, OracleError> {
            self.apys
                .read()
                .await
                .get(&(asset.clone(), protocol.clone()))
                .cloned()
                .ok_or_else(|| OracleError::UnsupportedAsset(format!("{:?}", asset)))
        }
        
        async fn get_liquidity(&self, asset: &Asset, protocol: &Protocol) -> Result<LiquidityData, OracleError> {
            self.liquidity
                .read()
                .await
                .get(&(asset.clone(), protocol.clone()))
                .cloned()
                .ok_or_else(|| OracleError::UnsupportedAsset(format!("{:?}", asset)))
        }
        
        async fn get_prices(&self, assets: &[Asset]) -> Result<HashMap<Asset, PriceData>, OracleError> {
            let mut prices = HashMap::new();
            let stored = self.prices.read().await;
            
            for asset in assets {
                if let Some(price) = stored.get(asset) {
                    prices.insert(asset.clone(), price.clone());
                } else {
                    return Err(OracleError::UnsupportedAsset(format!("{:?}", asset)));
                }
            }
            
            Ok(prices)
        }
        
        async fn get_apys(&self, assets: &[Asset], protocol: &Protocol) -> Result<HashMap<Asset, ApyData>, OracleError> {
            let mut apys = HashMap::new();
            let stored = self.apys.read().await;
            
            for asset in assets {
                if let Some(apy) = stored.get(&(asset.clone(), protocol.clone())) {
                    apys.insert(asset.clone(), apy.clone());
                } else {
                    return Err(OracleError::UnsupportedAsset(format!("{:?}", asset)));
                }
            }
            
            Ok(apys)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::block_on;
    
    #[test]
    fn test_mock_oracle() {
        block_on(async {
            let oracle = mock::MockOracle::new();
            
            // Set up test data
            let eth = Asset::Token("ETH".to_string());
            let aave = Protocol::Aave;
            
            let price_data = PriceData {
                price: U128(1_500_000_000_000),
                timestamp: 1234567890,
                source: "mock".to_string(),
            };
            
            let apy_data = ApyData {
                apy: 0.05,
                timestamp: 1234567890,
                protocol: Protocol::Aave,
                risk_score: 2,
            };
            
            // Test price fetching
            oracle.set_price(eth.clone(), price_data.clone()).await;
            let fetched_price = oracle.get_price(&eth).await.unwrap();
            assert_eq!(fetched_price.price, price_data.price);
            
            // Test APY fetching
            oracle.set_apy(eth.clone(), aave.clone(), apy_data.clone()).await;
            let fetched_apy = oracle.get_apy(&eth, &aave).await.unwrap();
            assert_eq!(fetched_apy.apy, apy_data.apy);
            
            // Test error handling
            let unknown_asset = Asset::Token("UNKNOWN".to_string());
            assert!(matches!(
                oracle.get_price(&unknown_asset).await,
                Err(OracleError::UnsupportedAsset(_))
            ));
        });
    }
} 