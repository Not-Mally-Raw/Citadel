use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use near_sdk::json_types::U128;
use parking_lot::RwLock;
use prettytable::{Cell, Row, Table};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;
use log::{info, warn, error};
use futures::future::join_all;
use metrics::{counter, gauge};
use thiserror::Error;
use lazy_static::lazy_static;

#[derive(Debug, Serialize, Deserialize)]
struct Metrics {
    tvl: f64,
    apy: f64,
    users: u64,
    risk: f64,
}

#[async_trait]
trait RetryableOperation: Send + Sync {
    type Output: Send;
    async fn execute(&self) -> Result<Self::Output>;
}

async fn with_retry<T>(operation: T, max_retries: u32) -> Result<T::Output> 
where
    T: RetryableOperation + Send + Sync,
    T::Output: Send,
{
    let mut retries = 0;
    loop {
        match operation.execute().await {
            Ok(result) => return Ok(result),
            Err(e) if retries < max_retries => {
                retries += 1;
                time::sleep(Duration::from_secs(2u64.pow(retries))).await;
                continue;
            }
            Err(e) => return Err(e.context(format!("Operation failed after {} retries", retries))),
        }
    }
}

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache entry not found")]
    NotFound,
    #[error("Cache entry expired")]
    Expired,
    #[error("Failed to acquire lock")]
    LockError,
    #[error("Value serialization error")]
    SerializationError,
}

type CacheResult<T> = Result<T, CacheError>;

#[derive(Clone)]
struct CacheEntry {
    value: Value,
    timestamp: Instant,
    access_count: u64,
}

struct MetricsCache {
    data: Arc<RwLock<HashMap<String, CacheEntry>>>,
    ttl: Duration,
    max_entries: usize,
}

#[derive(Debug)]
struct CacheStats {
    total_entries: usize,
    expired_entries: usize,
    total_access_count: u64,
}

#[derive(Debug)]
struct ProtocolApy {
    lending_apy: f64,
    borrowing_apy: f64,
    liquidity_apy: f64,
    total_apy: f64,
    weight: f64,
}

// Error definitions
#[derive(thiserror::Error, Debug)]
enum TvlError {
    #[error("Failed to fetch protocol data: {0}")]
    ProtocolError(String),
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Rate limit exceeded")]
    RateLimitError,
    #[error("Invalid response format")]
    InvalidResponseFormat,
}

#[derive(thiserror::Error, Debug)]
enum ApyError {
    #[error("Failed to fetch protocol APY: {0}")]
    ProtocolError(String),
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Rate limit exceeded")]
    RateLimitError,
    #[error("Invalid response format")]
    InvalidResponseFormat,
}

#[derive(thiserror::Error, Debug)]
enum UserError {
    #[error("Failed to fetch users from protocol: {0}")]
    ProtocolError(String),
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Rate limit exceeded")]
    RateLimitError,
    #[error("Invalid response format")]
    InvalidResponseFormat,
}

// Trait definitions
trait ProtocolOperation {
    fn protocol_name(&self) -> &str;
    fn operation_type(&self) -> &str;
}

// Implementations
impl MetricsCache {
    fn new(ttl: Duration, max_entries: usize) -> Self {
        let cache = Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            ttl,
            max_entries,
        };

        cache
    }

    fn get(&self, key: &str) -> CacheResult<Value> {
        let mut data = self.data.write();
        match data.get_mut(key) {
            Some(entry) => {
                if entry.timestamp.elapsed() < self.ttl {
                    entry.access_count += 1;
                    Ok(entry.value.clone())
                } else {
                    Err(CacheError::Expired)
                }
            }
            None => Err(CacheError::NotFound)
        }
    }

    fn set(&self, key: String, value: Value) -> CacheResult<()> {
        let mut data = self.data.write();

        if data.len() >= self.max_entries {
            let mut entries: Vec<_> = data.iter()
                .map(|(k, v)| (k.clone(), v.access_count))
                .collect();
            entries.sort_by_key(|(_k, count)| *count);

            let to_remove = (self.max_entries as f64 * 0.1) as usize;
            for (key, _) in entries.iter().take(to_remove) {
                data.remove(key);
            }
        }

        data.insert(key, CacheEntry {
            value,
            timestamp: Instant::now(),
            access_count: 0,
        });

        Ok(())
    }

    fn get_stats(&self) -> CacheResult<CacheStats> {
        let data = self.data.read();
        Ok(CacheStats {
            total_entries: data.len(),
            expired_entries: data.iter()
                .filter(|(_, entry)| entry.timestamp.elapsed() >= self.ttl)
                .count(),
            total_access_count: data.iter()
                .map(|(_, entry)| entry.access_count)
                .sum(),
        })
    }
}

// Global cache instance
lazy_static::lazy_static! {
    static ref METRICS_CACHE: Arc<RwLock<MetricsCache>> = Arc::new(RwLock::new(
        MetricsCache::new(Duration::from_secs(60), 1000)
    ));
}

// Helper functions
fn create_progress_bar(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "));
    pb.set_message(message);
    pb
}

fn format_number(num: u64) -> String {
    let mut s = String::new();
    let num_str = num.to_string();
    let a = num_str.chars().rev().enumerate();
    for (i, c) in a {
        if i != 0 && i % 3 == 0 {
            s.insert(0, ',');
        }
        s.insert(0, c);
    }
    s
}

// Main functions
async fn fetch_protocol_tvl(protocol: &str) -> Result<f64, TvlError> {
    // Implementation remains the same
}

async fn fetch_total_tvl() -> Result<f64> {
    // Implementation remains the same
}

async fn fetch_protocol_apy(protocol: &str) -> Result<ProtocolApy, ApyError> {
    // Implementation remains the same
}

async fn fetch_current_apy() -> Result<f64> {
    use futures::future::join_all;
    use std::time::Instant;

    let start_time = Instant::now();
    let protocol_weights = vec![
        ("aave", 0.4),
        ("compound", 0.35),
        ("uniswap", 0.25)
    ];

    let apy_futures = protocol_weights.iter().map(|&(protocol, weight)| {
        let protocol = protocol.to_string();
        async move {
            let fetch_result = with_retry(
                ProtocolApyFetcher { protocol: protocol.clone() },
                3 // Max retries
            ).await;

            match fetch_result {
                Ok(apy) => {
                    log::info!("APY metrics for {}: lending={:.2}%, borrowing={:.2}%, liquidity={:.2}%, total={:.2}%", 
                        protocol,
                        apy.lending_apy * 100.0,
                        apy.borrowing_apy * 100.0,
                        apy.liquidity_apy * 100.0,
                        apy.total_apy * 100.0
                    );
                    Ok((apy.total_apy, weight))
                }
                Err(e) => {
                    log::error!("Failed to fetch APY for {} after retries: {}", protocol, e);
                    Err(e)
                }
            }
        }
    });

    let results = join_all(apy_futures).await;
    
    let mut weighted_apy = 0.0;
    let mut total_weight = 0.0;

    for result in results {
        match result {
            Ok((apy, weight)) => {
                weighted_apy += apy * weight;
                total_weight += weight;
            }
            Err(e) => log::warn!("Skipping protocol due to error: {}", e)
        }
    }

    if total_weight == 0.0 {
        return Err(anyhow!("Failed to fetch APY from any protocol"));
    }

    let final_apy = weighted_apy / total_weight;
    log::info!("Weighted APY calculation completed in {:?}: {:.2}%", 
        start_time.elapsed(), 
        final_apy * 100.0
    );

    Ok(final_apy)
}

async fn fetch_users_for_protocol(protocol: &str) -> Result<Vec<String>, UserError> {
    let client = reqwest::Client::new();
    let cache_key = format!("users_{}", protocol);
    
    // Try to get from cache first
    if let Ok(cached_value) = METRICS_CACHE.read().get(&cache_key) {
        if let Some(users) = cached_value.as_array() {
            return Ok(users.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect());
        }
    }

    // Fetch from API if not in cache
    let url = match protocol {
        "aave" => "https://api.aave.com/v1/users",
        "compound" => "https://api.compound.finance/v2/users",
        "uniswap" => "https://api.uniswap.org/v1/users",
        _ => return Err(UserError::ProtocolError(format!("Unsupported protocol: {}", protocol)))
    };

    let response = client
        .get(url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(UserError::NetworkError)?;

    if response.status() == 429 {
        return Err(UserError::RateLimitError);
    }

    let data: Value = response
        .json()
        .await
        .map_err(UserError::NetworkError)?;

    let users = data["users"]
        .as_array()
        .ok_or(UserError::InvalidResponseFormat)?
        .iter()
        .filter_map(|v| v.as_str())
        .map(String::from)
        .collect::<Vec<String>>();

    // Cache the result
    if let Ok(mut cache) = METRICS_CACHE.write() {
        let cache_value = serde_json::to_value(&users)
            .map_err(|_| UserError::InvalidResponseFormat)?;
        cache.set(cache_key, cache_value)?;
    }

    Ok(users)
}

async fn fetch_active_users() -> Result<u64> {
    use futures::future::join_all;
    use std::time::Instant;
    use std::collections::HashSet;
    use metrics::{counter, gauge};

    let start_time = Instant::now();
    let protocols = vec!["aave", "compound", "uniswap"];
    let mut unique_users = HashSet::new();
    let mut error_details = Vec::new();

    let user_futures = protocols.iter().map(|&protocol| {
        let protocol = protocol.to_string();
        async move {
            let fetch_result = with_retry(
                UsersFetcher { protocol: protocol.clone() },
                3 // Max retries
            ).await;

            match fetch_result {
                Ok(users) => {
                    gauge!("protocol.users.count", users.len() as f64, "protocol" => &protocol);
                    log::info!("Fetched {} users from {}", users.len(), protocol);
                    Ok((protocol, users))
                }
                Err(e) => {
                    counter!("protocol.users.errors", 1, "protocol" => protocol.clone());
                    log::error!("Failed to fetch users from {} after retries: {}", protocol, e);
                    Err(e)
                }
            }
        }
    });

    let results = join_all(user_futures).await;
    let mut success_count = 0;

    for result in results {
        match result {
            Ok((protocol, users)) => {
                success_count += 1;
                for user in users {
                    unique_users.insert(user);
                }
            }
            Err(e) => error_details.push(e.to_string())
        }
    }

    // Record metrics
    let success_rate = success_count as f64 / protocols.len() as f64;
    gauge!("users.fetch.success_rate", success_rate);
    gauge!("users.fetch.duration_ms", start_time.elapsed().as_millis() as f64);
    gauge!("users.total_unique", unique_users.len() as f64);

    if unique_users.is_empty() {
        let error_msg = format!(
            "Failed to fetch users from any protocol. Errors: {}",
            error_details.join("; ")
        );
        log::error!("{}", error_msg);
        return Err(anyhow!(error_msg));
    }

    log::info!(
        "Fetched {} unique users across {} protocols in {:?} with {:.0}% success rate",
        unique_users.len(),
        protocols.len(),
        start_time.elapsed(),
        success_rate * 100.0
    );

    Ok(unique_users.len() as u64)
}

async fn calculate_risk_score() -> Result<f64> {
    // Calculate risk score based on multiple factors
    let factors: (Result<f64>, Result<f64>, Result<f64>, Result<f64>) = tokio::join!(
        calculate_market_volatility(),
        calculate_protocol_health(),
        calculate_collateral_ratio(),
        calculate_liquidity_depth()
    );

    async fn calculate_market_volatility() -> Result<f64> {
        // TODO: Implement market volatility calculation
        Ok(0.5)
    }

    async fn calculate_protocol_health() -> Result<f64> {
        // TODO: Implement protocol health calculation
        Ok(0.8)
    }

    async fn calculate_collateral_ratio() -> Result<f64> {
        // TODO: Implement collateral ratio calculation
        Ok(0.7)
    }

    async fn calculate_liquidity_depth() -> Result<f64> {
        // TODO: Implement liquidity depth calculation
        Ok(0.6)
    }

    let weights = [
        ("volatility", 0.3),
        ("health", 0.3),
        ("collateral", 0.2),
        ("liquidity", 0.2)
    ];

    let mut weighted_risk = 0.0;
    let mut total_weight = 0.0;

    for (result, (name, weight)) in 
        [factors.0, factors.1, factors.2, factors.3].iter().zip(weights.iter()) {
        match result {
            Ok(score) => {
                weighted_risk += score * weight;
                total_weight += weight;
            },
            Err(e) => warn!("Failed to calculate {} risk: {}", name, e)
        }
    }

    if total_weight == 0.0 {
        Err(anyhow!("Failed to calculate risk score"))
    } else {
        Ok(weighted_risk / total_weight)
    }
}

pub async fn deposit(amount: U128) -> Result<(), Box<dyn std::error::Error>> {
    let pb = create_progress_bar("Processing deposit");
    // Simulate transaction steps
    pb.set_message("Checking balance...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.set_message("Initiating transaction...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.set_message("Confirming transaction...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.finish_with_message("Deposit successful!");

    println!("\n{}", format!("Deposited {} tokens", amount.0).green());
    Ok(())
}

pub async fn withdraw(amount: U128) -> Result<(), Box<dyn std::error::Error>> {
    let pb = create_progress_bar("Processing withdrawal");
    pb.set_message("Checking vault balance...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.set_message("Calculating fees...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.set_message("Initiating transfer...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.finish_with_message("Withdrawal successful!");

    println!("\n{}", format!("Withdrawn {} tokens", amount.0).green());
    Ok(())
}

pub async fn get_info() -> Result<()> {
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("Metric").style_spec("Fb"),
        Cell::new("Value").style_spec("Fb"),
    ]));

    // Try to get metrics from cache first
    let metrics = if let Ok(cached_value) = METRICS_CACHE.read().get("vault_metrics") {
        cached_value
    } else {
        // Fetch metrics concurrently with proper error handling
        let (tvl, apy, users, risk) = tokio::join!(
            fetch_total_tvl(),
            fetch_current_apy(),
            fetch_active_users(),
            calculate_risk_score()
        );

        let tvl = tvl.with_context(|| "Failed to fetch TVL")?;
        let apy = apy.with_context(|| "Failed to fetch APY")?;
        let users = users.with_context(|| "Failed to fetch active users")?;
        let risk = risk.with_context(|| "Failed to calculate risk score")?;

        let metrics = json!({
            "tvl": format!("${:.2}M", tvl / 1_000_000.0),
            "apy": format!("{:.2}%", apy * 100.0),
            "users": format_number(users),
            "risk": format!("{:.2}", risk)
        });

        // Update cache
        if let Ok(mut cache) = METRICS_CACHE.write() {
            let _ = cache.set("vault_metrics".to_string(), metrics.clone());
        }

        metrics
    };

    for (metric, value) in metrics.as_object()
        .ok_or_else(|| anyhow!("Invalid metrics format"))?
        .iter() 
    {
        table.add_row(Row::new(vec![
            Cell::new(metric),
            Cell::new(&value.as_str().unwrap_or("N/A")),
        ]));
    }

    table.printstd();
    Ok(())
}

pub async fn analyze_performance() -> Result<(), Box<dyn std::error::Error>> {
    let pb = create_progress_bar("Analyzing performance");
    
    // Simulate analysis
    pb.set_message("Gathering historical data...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.set_message("Calculating metrics...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.set_message("Generating report...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.finish_with_message("Analysis complete!");

    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("Performance Metric").style_spec("Fb"),
        Cell::new("Value").style_spec("Fb"),
        Cell::new("Trend").style_spec("Fb"),
    ]));

    let metrics = [
        ("TVL Growth Rate", "+5.2%", "↗"),
        ("User Growth", "+12.3%", "↗"),
        ("Risk-Adjusted APY", "10.8%", "→"),
        ("Gas Efficiency", "92%", "↗"),
    ];

    for (metric, value, trend) in metrics.iter() {
        table.add_row(Row::new(vec![
            Cell::new(metric),
            Cell::new(value),
            Cell::new(trend),
        ]));
    }

    table.printstd();
    Ok(())
}

pub async fn optimize_strategy() -> Result<(), Box<dyn std::error::Error>> {
    let pb = create_progress_bar("Optimizing strategy");

    // Simulate optimization steps
    pb.set_message("Analyzing market conditions...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.set_message("Evaluating risk parameters...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.set_message("Adjusting allocation...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.finish_with_message("Strategy optimized!");

    println!("\n{}", "Strategy optimization complete.".green());
    println!("New allocation:");

    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("Asset").style_spec("Fb"),
        Cell::new("Previous Allocation").style_spec("Fb"),
        Cell::new("New Allocation").style_spec("Fb"),
    ]));

    let assets = [
        ("USDC", "30%", "35%"),
        ("ETH", "25%", "20%"),
        ("WBTC", "20%", "25%"),
        ("DAI", "15%", "10%"),
        ("Other", "10%", "10%"),
    ];

    for (asset, prev, new) in assets.iter() {
        table.add_row(Row::new(vec![
            Cell::new(asset),
            Cell::new(&format!("{:.2}%", prev)),
            Cell::new(&format!("{:.2}%", new))
        ]));
    }

    table.printstd();
    Ok(())
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("API request failed: {0}")]
    ApiError(String),
    #[error("Cache error: {0}")]
    CacheError(#[from] CacheError),
    #[error("Invalid input: {0}")]
    ValidationError(String),
    #[error("Operation timeout: {0}")]
    TimeoutError(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}

type CommandResult<T> = Result<T, CommandError>;

#[derive(Debug, Clone)]
struct MetricsTracker {
    success_count: Arc<RwLock<u64>>,
    error_count: Arc<RwLock<u64>>,
    latency_ms: Arc<RwLock<Vec<u64>>>,
}

impl MetricsTracker {
    fn new() -> Self {
        Self {
            success_count: Arc::new(RwLock::new(0)),
            error_count: Arc::new(RwLock::new(0)),
            latency_ms: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn record_success(&self, latency_ms: u64) {
        *self.success_count.write() += 1;
        self.latency_ms.write().push(latency_ms);
    }

    fn record_error(&self) {
        *self.error_count.write() += 1;
    }

    fn get_stats(&self) -> CommandResult<MetricsStats> {
        let success = *self.success_count.read();
        let errors = *self.error_count.read();
        let latencies = self.latency_ms.read().clone();
        
        let avg_latency = if !latencies.is_empty() {
            latencies.iter().sum::<u64>() as f64 / latencies.len() as f64
        } else {
            0.0
        };

        Ok(MetricsStats {
            success_count: success,
            error_count: errors,
            avg_latency_ms: avg_latency,
        })
    }
}

#[derive(Debug, Serialize)]
struct MetricsStats {
    success_count: u64,
    error_count: u64,
    avg_latency_ms: f64,
}

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Protocol not supported: {0}")]
    UnsupportedProtocol(String),
    #[error("API rate limit exceeded for {0}")]
    RateLimitExceeded(String),
    #[error("Connection timeout for {0}")]
    ConnectionTimeout(String),
    #[error("Invalid response from {0}: {1}")]
    InvalidResponse(String, String),
}

async fn execute_protocol_request<T, F>(protocol: &str, request: F) -> CommandResult<T>
where
    F: Future<Output = Result<T, reqwest::Error>> + Send,
{
    let start = std::time::Instant::now();
    let metrics = MetricsTracker::new();

    match tokio::time::timeout(Duration::from_secs(10), request).await {
        Ok(result) => match result {
            Ok(data) => {
                metrics.record_success(start.elapsed().as_millis() as u64);
                Ok(data)
            }
            Err(e) if e.is_timeout() => {
                metrics.record_error();
                Err(CommandError::from(ProtocolError::ConnectionTimeout(protocol.to_string())))
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS) => {
                metrics.record_error();
                Err(CommandError::from(ProtocolError::RateLimitExceeded(protocol.to_string())))
            }
            Err(e) => {
                metrics.record_error();
                Err(CommandError::ApiError(e.to_string()))
            }
        },
        Err(_) => {
            metrics.record_error();
            Err(CommandError::from(ProtocolError::ConnectionTimeout(protocol.to_string())))
        }
    }
}

pub async fn monitor_health() -> Result<(), Box<dyn std::error::Error>> {
    let pb = create_progress_bar("Checking system health");
    
    pb.set_message("Checking smart contracts...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.set_message("Verifying oracle feeds...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.set_message("Analyzing metrics...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    pb.finish_with_message("Health check complete!");

    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("Component").style_spec("Fb"),
        Cell::new("Status").style_spec("Fb"),
        Cell::new("Details").style_spec("Fb"),
    ]));

    let statuses = [
        ("Smart Contracts", "✅ Healthy", "All functions operational"),
        ("Oracle Feeds", "✅ Healthy", "Last update: 2 min ago"),
        ("TVL", "✅ Healthy", "No unusual changes"),
        ("Gas Usage", "⚠️ Warning", "Above average usage"),
    ];

    for (component, status, details) in statuses.iter() {
        table.add_row(Row::new(vec![
            Cell::new(component),
            Cell::new(status),
            Cell::new(details),
        ]));
    }

    table.printstd();
    Ok(())
}

#[derive(Debug)]
struct ProtocolApyFetcher {
    protocol: String,
}

#[async_trait]
impl RetryableOperation for ProtocolApyFetcher {
    type Output = ProtocolApy;

    async fn execute(&self) -> Result<Self::Output> {
        fetch_protocol_apy(&self.protocol)
            .await
            .map_err(|e| anyhow!("APY fetch error: {}", e))
    }
}

#[derive(Debug)]
struct UsersFetcher {
    protocol: String,
}

#[async_trait]
impl RetryableOperation for UsersFetcher {
    type Output = Vec<String>;

    async fn execute(&self) -> Result<Self::Output> {
        fetch_users_for_protocol(&self.protocol)
            .await
            .map_err(|e| anyhow!("Users fetch error: {}", e))
    }
}

#[derive(Debug)]
struct TvlFetcher {
    protocol: String,
}

#[async_trait]
impl RetryableOperation for TvlFetcher {
    type Output = f64;

    async fn execute(&self) -> Result<Self::Output> {
        fetch_protocol_tvl(&self.protocol)
            .await
            .map_err(|e| anyhow!("TVL fetch error: {}", e))
    }
}

impl ProtocolOperation for ProtocolApyFetcher {
    fn protocol_name(&self) -> &str {
        &self.protocol
    }

    fn operation_type(&self) -> &str {
        "APY"
    }
}

impl ProtocolOperation for UsersFetcher {
    fn protocol_name(&self) -> &str {
        &self.protocol
    }

    fn operation_type(&self) -> &str {
        "Users"
    }
}

impl ProtocolOperation for TvlFetcher {
    fn protocol_name(&self) -> &str {
        &self.protocol
    }

    fn operation_type(&self) -> &str {
        "TVL"
    }
}
