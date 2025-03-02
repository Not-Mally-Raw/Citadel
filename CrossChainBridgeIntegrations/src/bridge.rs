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

// Constants for performance tuning
const MAX_BATCH_SIZE: usize = 50;
const MAX_CONCURRENT_TRANSFERS: usize = 10;
const CACHE_TTL: Duration = Duration::from_secs(60);
const MAX_RETRIES: u32 = 3;
const GAS_PRICE_CACHE_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub source_chain: String,
    pub target_chain: String,
    pub token_address: String,
    pub bridge_address: String,
    pub confirmation_blocks: u64,
    pub max_gas_price: u64,
    pub min_transfer_amount: u64,
    pub max_transfer_amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    pub sender: String,
    pub receiver: String,
    pub token: String,
    pub amount: u64,
    pub deadline: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStatus {
    pub tx_hash: String,
    pub from_chain: String,
    pub to_chain: String,
    pub amount: u64,
    pub timestamp: u64,
    pub status: TransferState,
    pub retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferState {
    Pending,
    Confirming,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub decimals: u8,
    pub symbol: String,
    pub total_supply: u64,
    pub cached_at: std::time::Instant,
}

pub struct Bridge {
    config: BridgeConfig,
    pending_transfers: Arc<DashMap<String, TransferStatus>>,
    token_cache: Arc<RwLock<LruCache<String, TokenInfo>>>,
    gas_price_cache: Arc<RwLock<(u64, std::time::Instant)>>,
    transfer_semaphore: Arc<Semaphore>,
    metrics: Arc<Metrics>,
}

#[derive(Debug)]
struct Metrics {
    total_transfers: metrics::Counter,
    failed_transfers: metrics::Counter,
    active_transfers: metrics::Gauge,
    average_confirmation_time: metrics::Gauge,
    gas_price: metrics::Gauge,
}

impl Bridge {
    pub fn new(config: BridgeConfig) -> Self {
        let metrics = Arc::new(Metrics {
            total_transfers: counter!("bridge_total_transfers"),
            failed_transfers: counter!("bridge_failed_transfers"),
            active_transfers: gauge!("bridge_active_transfers"),
            average_confirmation_time: gauge!("bridge_avg_confirmation_time"),
            gas_price: gauge!("bridge_gas_price"),
        });

        Self {
            config,
            pending_transfers: Arc::new(DashMap::new()),
            token_cache: Arc::new(RwLock::new(LruCache::new(100))),
            gas_price_cache: Arc::new(RwLock::new((0, std::time::Instant::now()))),
            transfer_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_TRANSFERS)),
            metrics,
        }
    }

    pub async fn batch_transfer(&self, transfers: Vec<TransferRequest>) -> Result<Vec<String>> {
        // Validate batch size
        if transfers.is_empty() {
            return Ok(Vec::new());
        }

        // Split into optimal batch sizes
        let mut tx_hashes = Vec::new();
        let mut current_batch = Vec::new();

        for transfer in transfers {
            if current_batch.len() >= MAX_BATCH_SIZE {
                let batch_hashes = self.process_transfer_batch(&current_batch).await?;
                tx_hashes.extend(batch_hashes);
                current_batch.clear();
            }
            current_batch.push(transfer);
        }

        // Process remaining transfers
        if !current_batch.is_empty() {
            let batch_hashes = self.process_transfer_batch(&current_batch).await?;
            tx_hashes.extend(batch_hashes);
        }

        Ok(tx_hashes)
    }

    async fn process_transfer_batch(&self, batch: &[TransferRequest]) -> Result<Vec<String>> {
        let _permit = self.transfer_semaphore.acquire().await?;
        
        // Get current gas price once for the batch
        let gas_price = self.get_current_gas_price().await?;
        
        // Process transfers in parallel with bounded concurrency
        let results: Vec<Result<String>> = stream::iter(batch)
            .map(|transfer| self.execute_single_transfer(transfer, gas_price))
            .buffer_unordered(MAX_CONCURRENT_TRANSFERS)
            .collect()
            .await;

        // Collect successful transfers and log errors
        let mut tx_hashes = Vec::new();
        for result in results {
            match result {
                Ok(hash) => tx_hashes.push(hash),
                Err(e) => {
                    error!("Transfer failed: {}", e);
                    self.metrics.failed_transfers.increment(1);
                }
            }
        }

        Ok(tx_hashes)
    }

    async fn execute_single_transfer(&self, transfer: &TransferRequest, gas_price: u64) -> Result<String> {
        // Validate transfer
        self.validate_transfer(transfer).await?;

        // Get cached token info or fetch it
        let token_info = self.get_token_info(&transfer.token).await?;

        // Execute transfer with retry logic
        let tx_hash = self.execute_transfer_with_retry(transfer, &token_info, gas_price).await?;

        // Update metrics
        self.metrics.total_transfers.increment(1);
        self.metrics.active_transfers.increment(1);

        Ok(tx_hash)
    }

    async fn execute_transfer_with_retry(
        &self,
        transfer: &TransferRequest,
        token_info: &TokenInfo,
        gas_price: u64,
    ) -> Result<String> {
        let mut backoff = backoff::ExponentialBackoff::default();
        backoff.max_elapsed_time = Some(Duration::from_secs(300)); // 5 minutes max

        backoff::future::retry(backoff, || async {
            match self.execute_transfer_internal(transfer, token_info, gas_price).await {
                Ok(hash) => Ok(hash),
                Err(e) => {
                    warn!("Transfer retry needed: {}", e);
                    Err(backoff::Error::transient(e))
                }
            }
        })
        .await
    }

    async fn get_current_gas_price(&self) -> Result<u64> {
        let cache = self.gas_price_cache.read();
        let (cached_price, cached_at) = (*cache.0, cache.1);
        
        if cached_at.elapsed() < GAS_PRICE_CACHE_TTL {
            return Ok(cached_price);
        }
        drop(cache);

        // Fetch new gas price
        let new_price = self.fetch_gas_price().await?;
        *self.gas_price_cache.write() = (new_price, std::time::Instant::now());
        self.metrics.gas_price.set(new_price as f64);
        
        Ok(new_price)
    }

    async fn get_token_info(&self, token_address: &str) -> Result<TokenInfo> {
        // Try to get from cache first
        if let Some(info) = self.token_cache.read().get(token_address) {
            if info.cached_at.elapsed() < CACHE_TTL {
                return Ok(info.clone());
            }
        }

        // Fetch fresh token info
        let info = self.fetch_token_info(token_address).await?;
        self.token_cache.write().put(token_address.to_string(), info.clone());

        Ok(info)
    }

    pub async fn monitor_pending_transfers(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));

        loop {
            interval.tick().await;
            self.process_pending_transfers().await;
        }
    }

    async fn process_pending_transfers(&self) {
        let pending: Vec<_> = self.pending_transfers
            .iter()
            .filter(|r| r.value().status == TransferState::Pending)
            .map(|r| r.key().clone())
            .collect();

        stream::iter(pending)
            .for_each_concurrent(MAX_CONCURRENT_TRANSFERS, |tx_hash| async move {
                if let Err(e) = self.check_transfer_status(&tx_hash).await {
                    error!("Failed to check transfer status: {}", e);
                }
            })
            .await;
    }

    async fn validate_transfer(&self, transfer: &TransferRequest) -> Result<()> {
        // Amount validation
        if transfer.amount < self.config.min_transfer_amount {
            return Err(anyhow::anyhow!("Transfer amount below minimum"));
        }
        if transfer.amount > self.config.max_transfer_amount {
            return Err(anyhow::anyhow!("Transfer amount above maximum"));
        }

        // Deadline validation
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        if transfer.deadline <= current_time {
            return Err(anyhow::anyhow!("Transfer deadline expired"));
        }

        Ok(())
    }

    // Placeholder methods that need to be implemented based on specific chain requirements
    async fn execute_transfer_internal(&self, transfer: &TransferRequest, token_info: &TokenInfo, gas_price: u64) -> Result<String> {
        unimplemented!("Implement chain-specific transfer logic")
    }

    async fn fetch_token_info(&self, token_address: &str) -> Result<TokenInfo> {
        unimplemented!("Implement chain-specific token info fetching")
    }

    async fn fetch_gas_price(&self) -> Result<u64> {
        unimplemented!("Implement chain-specific gas price fetching")
    }

    async fn check_transfer_status(&self, tx_hash: &str) -> Result<()> {
        unimplemented!("Implement chain-specific status checking")
    }
}
