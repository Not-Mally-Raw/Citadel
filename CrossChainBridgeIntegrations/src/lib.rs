use anyhow::{Context, Result};
use dashmap::DashMap;
use futures::{stream, StreamExt};
use lru::LruCache;
use metrics::{counter, gauge};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{info, warn, error};

// Constants
const MAX_BATCH_SIZE: usize = 50;
const MAX_CONCURRENT_TRANSFERS: usize = 10;
const CACHE_TTL: Duration = Duration::from_secs(60);
const MAX_RETRIES: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub chain_id: u64,
    pub bridge_address: String,
    pub token_address: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let config = BridgeConfig {
            chain_id: 1,
            bridge_address: "bridge.near".to_string(),
            token_address: "token.near".to_string(),
        };
        assert_eq!(config.chain_id, 1);
    }
} 