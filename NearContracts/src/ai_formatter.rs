use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use super::analytics::{PoolMetrics, APYBreakdown, RiskScore, VolatilityMetrics, EnhancedPoolMetrics, VolatilityRegime, Signal};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct AIModelInput {
    // Core Features
    pub pool_features: PoolFeatures,
    pub market_features: MarketFeatures,
    pub risk_features: RiskFeatures,
    pub temporal_features: TemporalFeatures,
    
    // Target Variables
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolFeatures {
    pub tvl_normalized: f64,
    pub volume_to_tvl_ratio: f64,
    pub liquidity_depth: f64,
    pub token_correlation: f64,
    pub pool_age_days: u32,
    pub pool_type_encoding: Vec<f64>,
    pub platform_encoding: Vec<f64>,
    pub chain_encoding: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketFeatures {
    pub price_volatility_1d: f64,
    pub price_volatility_7d: f64,
    pub price_volatility_30d: f64,
    pub volume_trend: f64,
    pub tvl_trend: f64,
    pub market_correlation: f64,
    pub token_dominance: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskFeatures {
    pub impermanent_loss_risk: f64,
    pub volatility_risk: f64,
    pub security_risk: f64,
    pub concentration_risk: f64,
    pub smart_contract_risk: f64,
    pub historical_risk_events: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemporalFeatures {
    pub time_series: Vec<TimeSeriesPoint>,
    pub seasonality: Vec<f64>,
    pub trend_indicators: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: u64,
    pub tvl: f64,
    pub volume: f64,
    pub apy: f64,
    pub il: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub realized_apy: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub max_drawdown: f64,
    pub success_rate: f64,
}

impl From<&PoolMetrics> for AIModelInput {
    fn from(metrics: &PoolMetrics) -> Self {
        Self {
            pool_features: PoolFeatures::from(metrics),
            market_features: MarketFeatures::from(metrics),
            risk_features: RiskFeatures::from(metrics),
            temporal_features: TemporalFeatures::from(metrics),
            performance_metrics: PerformanceMetrics::from(metrics),
        }
    }
}

impl PoolFeatures {
    pub fn from(metrics: &PoolMetrics) -> Self {
        let tvl = metrics.tvl as f64;
        let volume = metrics.volume_24h as f64;
        
        Self {
            tvl_normalized: normalize_tvl(tvl),
            volume_to_tvl_ratio: if tvl > 0.0 { volume / tvl } else { 0.0 },
            liquidity_depth: calculate_liquidity_depth(metrics),
            token_correlation: metrics.impermanent_loss_risk.price_correlation.to_f64().unwrap_or(0.0),
            pool_age_days: ((metrics.creation_timestamp - std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()) / 86400) as u32,
            pool_type_encoding: encode_pool_type(&metrics.pool_type),
            platform_encoding: encode_platform(&metrics.platform),
            chain_encoding: encode_chain(&metrics.chain),
        }
    }
}

impl MarketFeatures {
    pub fn from(metrics: &PoolMetrics) -> Self {
        Self {
            price_volatility_1d: metrics.market_volatility.daily_volatility.to_f64().unwrap_or(0.0),
            price_volatility_7d: metrics.market_volatility.weekly_volatility.to_f64().unwrap_or(0.0),
            price_volatility_30d: metrics.market_volatility.monthly_volatility.to_f64().unwrap_or(0.0),
            volume_trend: calculate_volume_trend(&metrics.performance_history.volume_history),
            tvl_trend: calculate_tvl_trend(&metrics.performance_history.tvl_history),
            market_correlation: calculate_market_correlation(metrics),
            token_dominance: calculate_token_dominance(&metrics.token_distribution),
        }
    }
}

impl RiskFeatures {
    pub fn from(metrics: &PoolMetrics) -> Self {
        Self {
            impermanent_loss_risk: metrics.impermanent_loss_risk.score as f64 / 100.0,
            volatility_risk: metrics.market_volatility.volatility_rank as f64 / 100.0,
            security_risk: metrics.security_score.contract_risk as f64 / 100.0,
            concentration_risk: metrics.user_metrics.user_concentration.to_f64().unwrap_or(0.0),
            smart_contract_risk: calculate_smart_contract_risk(metrics),
            historical_risk_events: encode_risk_events(&metrics.security_score.exploit_history),
        }
    }
}

impl TemporalFeatures {
    pub fn from(metrics: &PoolMetrics) -> Self {
        Self {
            time_series: create_time_series(metrics),
            seasonality: calculate_seasonality(metrics),
            trend_indicators: calculate_trend_indicators(metrics),
        }
    }
}

// Helper functions
fn normalize_tvl(tvl: f64) -> f64 {
    // Log normalization with scaling
    if tvl <= 0.0 {
        0.0
    } else {
        (tvl.ln() / 25.0).min(1.0)  // 25.0 ~= ln(72B) for max TVL normalization
    }
}

fn calculate_liquidity_depth(metrics: &PoolMetrics) -> f64 {
    let price_impact = metrics.market_volatility.price_impact_10000usd.to_f64().unwrap_or(0.0);
    1.0 / (1.0 + price_impact)
}

fn encode_pool_type(pool_type: &PoolType) -> Vec<f64> {
    // One-hot encoding for pool types
    vec![
        if matches!(pool_type, PoolType::Stable) { 1.0 } else { 0.0 },
        if matches!(pool_type, PoolType::Volatile) { 1.0 } else { 0.0 },
        if matches!(pool_type, PoolType::Weighted) { 1.0 } else { 0.0 },
        if matches!(pool_type, PoolType::Concentrated) { 1.0 } else { 0.0 },
        if matches!(pool_type, PoolType::Hybrid) { 1.0 } else { 0.0 },
    ]
}

fn calculate_volume_trend(history: &[(u64, Balance)]) -> f64 {
    if history.len() < 2 {
        return 0.0;
    }
    
    let recent = history.last().unwrap().1 as f64;
    let old = history.first().unwrap().1 as f64;
    
    if old == 0.0 {
        0.0
    } else {
        ((recent - old) / old).min(5.0).max(-5.0)
    }
}

fn calculate_market_correlation(metrics: &PoolMetrics) -> f64 {
    // Calculate correlation between pool returns and market returns
    let pool_returns: Vec<f64> = metrics.performance_history.daily_returns
        .iter()
        .map(|(_, r)| r.to_f64().unwrap_or(0.0))
        .collect();
    
    if pool_returns.is_empty() {
        return 0.0;
    }
    
    // Simplified market correlation calculation
    let mean = pool_returns.iter().sum::<f64>() / pool_returns.len() as f64;
    let std_dev = (pool_returns.iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>() / pool_returns.len() as f64)
        .sqrt();
    
    if std_dev == 0.0 {
        0.0
    } else {
        mean / std_dev
    }
}

fn calculate_token_dominance(tokens: &[TokenShare]) -> Vec<f64> {
    let total: Decimal = tokens.iter().map(|t| t.value_usd).sum();
    if total == Decimal::ZERO {
        return vec![0.0; tokens.len()];
    }
    
    tokens.iter()
        .map(|t| (t.value_usd / total).to_f64().unwrap_or(0.0))
        .collect()
}

fn create_time_series(metrics: &PoolMetrics) -> Vec<TimeSeriesPoint> {
    let mut series = Vec::new();
    let history = &metrics.performance_history;
    
    for i in 0..history.daily_returns.len() {
        let point = TimeSeriesPoint {
            timestamp: history.daily_returns[i].0,
            tvl: history.tvl_history.get(i).map(|&(_, tvl)| tvl as f64).unwrap_or(0.0),
            volume: history.volume_history.get(i).map(|&(_, vol)| vol as f64).unwrap_or(0.0),
            apy: history.daily_returns[i].1.to_f64().unwrap_or(0.0),
            il: history.il_history.get(i).map(|&(_, il)| il.to_f64().unwrap_or(0.0)).unwrap_or(0.0),
        };
        series.push(point);
    }
    
    series
}

fn calculate_seasonality(metrics: &PoolMetrics) -> Vec<f64> {
    // Calculate daily and weekly patterns
    let mut seasonality = vec![0.0; 24 + 7]; // 24 hours + 7 days
    
    // Process volume history for patterns
    for (timestamp, volume) in &metrics.performance_history.volume_history {
        let hour = (timestamp % 86400) / 3600;
        let day = (timestamp % 604800) / 86400;
        
        seasonality[hour as usize] += *volume as f64;
        seasonality[24 + day as usize] += *volume as f64;
    }
    
    // Normalize
    let max_value = seasonality.iter().fold(0.0, |a, &b| a.max(b));
    if max_value > 0.0 {
        seasonality.iter_mut().for_each(|v| *v /= max_value);
    }
    
    seasonality
}

fn calculate_trend_indicators(metrics: &PoolMetrics) -> Vec<f64> {
    let mut indicators = Vec::new();
    
    // TVL trend
    if let Some(tvl_trend) = calculate_trend(&metrics.performance_history.tvl_history) {
        indicators.push(tvl_trend);
    }
    
    // Volume trend
    if let Some(volume_trend) = calculate_trend(&metrics.performance_history.volume_history) {
        indicators.push(volume_trend);
    }
    
    // APY stability
    indicators.push(metrics.apy.apy_stability_score as f64 / 100.0);
    
    indicators
}

fn calculate_trend<T: Into<f64> + Copy>(history: &[(u64, T)]) -> Option<f64> {
    if history.len() < 2 {
        return None;
    }
    
    let x: Vec<f64> = (0..history.len()).map(|i| i as f64).collect();
    let y: Vec<f64> = history.iter().map(|(_, v)| (*v).into()).collect();
    
    let n = x.len() as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_xy: f64 = x.iter().zip(&y).map(|(&x, &y)| x * y).sum();
    let sum_xx: f64 = x.iter().map(|&x| x * x).sum();
    
    let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
    Some(slope)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedAIModelInput {
    // Core Features
    pub pool_features: EnhancedPoolFeatures,
    pub market_features: EnhancedMarketFeatures,
    pub risk_features: EnhancedRiskFeatures,
    pub temporal_features: EnhancedTemporalFeatures,
    
    // Advanced Features
    pub technical_indicators: TechnicalIndicators,
    pub market_sentiment: MarketSentiment,
    pub cross_chain_metrics: CrossChainMetrics,
    pub optimization_features: OptimizationFeatures,
    
    // Target Variables
    pub performance_metrics: EnhancedPerformanceMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedPoolFeatures {
    pub tvl_normalized: f64,
    pub volume_to_tvl_ratio: f64,
    pub liquidity_depth: f64,
    pub token_correlation: f64,
    pub pool_age_days: u32,
    pub pool_type_encoding: Vec<f64>,
    pub platform_encoding: Vec<f64>,
    pub chain_encoding: Vec<f64>,
    pub token_weights: Vec<f64>,
    pub pool_composition_score: f64,
    pub protocol_dominance: f64,
    pub capital_efficiency: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedMarketFeatures {
    pub price_volatility_1d: f64,
    pub price_volatility_7d: f64,
    pub price_volatility_30d: f64,
    pub volume_trend: f64,
    pub tvl_trend: f64,
    pub market_correlation: f64,
    pub token_dominance: Vec<f64>,
    pub market_regime: String,
    pub liquidity_score: f64,
    pub market_impact: f64,
    pub bid_ask_spread: f64,
    pub depth_analysis: MarketDepthAnalysis,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketDepthAnalysis {
    pub depth_2pct: f64,
    pub depth_5pct: f64,
    pub depth_10pct: f64,
    pub slippage_impact: f64,
    pub order_book_imbalance: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TechnicalIndicators {
    pub rsi_signals: Vec<f64>,
    pub macd_signals: Vec<f64>,
    pub bollinger_signals: Vec<f64>,
    pub momentum_signals: Vec<f64>,
    pub trend_strength: f64,
    pub support_resistance: Vec<f64>,
    pub volatility_regime: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketSentiment {
    pub social_volume: f64,
    pub sentiment_score: f64,
    pub developer_activity: f64,
    pub governance_participation: f64,
    pub market_fear_greed: f64,
    pub whale_activity: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrossChainMetrics {
    pub chain_tvl_share: HashMap<String, f64>,
    pub cross_chain_volume: HashMap<String, f64>,
    pub bridge_efficiency: HashMap<String, f64>,
    pub gas_adjusted_returns: HashMap<String, f64>,
    pub chain_correlation: Vec<Vec<f64>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OptimizationFeatures {
    pub optimal_position_size: f64,
    pub rebalance_signals: Vec<f64>,
    pub entry_points: Vec<f64>,
    pub exit_points: Vec<f64>,
    pub risk_adjusted_allocation: Vec<f64>,
    pub gas_optimization_score: f64,
    pub timing_efficiency: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedPerformanceMetrics {
    pub realized_apy: f64,
    pub risk_adjusted_return: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub max_drawdown: f64,
    pub recovery_factor: f64,
    pub win_loss_ratio: f64,
    pub profit_factor: f64,
    pub calmar_ratio: f64,
    pub omega_ratio: f64,
    pub var_95: f64,
    pub expected_shortfall: f64,
}

impl From<&EnhancedPoolMetrics> for EnhancedAIModelInput {
    fn from(metrics: &EnhancedPoolMetrics) -> Self {
        Self {
            pool_features: EnhancedPoolFeatures::from(metrics),
            market_features: EnhancedMarketFeatures::from(metrics),
            technical_indicators: TechnicalIndicators::from(metrics),
            market_sentiment: MarketSentiment::from(metrics),
            cross_chain_metrics: CrossChainMetrics::from(metrics),
            optimization_features: OptimizationFeatures::from(metrics),
            performance_metrics: EnhancedPerformanceMetrics::from(metrics),
            risk_features: EnhancedRiskFeatures::from(metrics),
            temporal_features: EnhancedTemporalFeatures::from(metrics),
        }
    }
}

impl EnhancedPoolFeatures {
    pub fn from(metrics: &EnhancedPoolMetrics) -> Self {
        let base = &metrics.base_metrics;
        let tvl = base.tvl as f64;
        
        Self {
            tvl_normalized: normalize_tvl(tvl),
            volume_to_tvl_ratio: calculate_volume_tvl_ratio(base),
            liquidity_depth: calculate_liquidity_depth(base),
            token_correlation: metrics.advanced_metrics.beta_coefficient.to_f64().unwrap_or(0.0),
            pool_age_days: calculate_pool_age(base),
            pool_type_encoding: encode_pool_type(&base.pool_type),
            platform_encoding: encode_platform(&base.platform),
            chain_encoding: encode_chain(&base.chain),
            token_weights: calculate_token_weights(&base.token_distribution),
            pool_composition_score: calculate_composition_score(base),
            protocol_dominance: calculate_protocol_dominance(base),
            capital_efficiency: calculate_capital_efficiency(metrics),
        }
    }
}

// Advanced calculation methods
fn calculate_capital_efficiency(metrics: &EnhancedPoolMetrics) -> f64 {
    let volume = metrics.base_metrics.volume_24h as f64;
    let tvl = metrics.base_metrics.tvl as f64;
    let utilization = metrics.ml_features.market_indicators.market_efficiency_coefficient.to_f64().unwrap_or(0.0);
    
    if tvl == 0.0 {
        return 0.0;
    }
    
    let efficiency = (volume / tvl) * utilization;
    efficiency.min(1.0)
}

fn calculate_protocol_dominance(metrics: &PoolMetrics) -> f64 {
    let total_tvl = metrics.tvl as f64;
    let platform_tvl = 1_000_000_000.0; // Example platform TVL, should be fetched from external source
    
    if platform_tvl == 0.0 {
        return 0.0;
    }
    
    (total_tvl / platform_tvl).min(1.0)
}

impl TechnicalIndicators {
    pub fn from(metrics: &EnhancedPoolMetrics) -> Self {
        let momentum = &metrics.ml_features.momentum_indicators;
        
        Self {
            rsi_signals: vec![momentum.rsi_14.to_f64().unwrap_or(50.0)],
            macd_signals: vec![
                momentum.macd.0.to_f64().unwrap_or(0.0),
                momentum.macd.1.to_f64().unwrap_or(0.0),
            ],
            bollinger_signals: calculate_bollinger_signals(&metrics.ml_features.volatility_indicators.bollinger_bands),
            momentum_signals: vec![momentum.momentum_score.to_f64().unwrap_or(0.0)],
            trend_strength: calculate_trend_strength(metrics),
            support_resistance: calculate_support_resistance(metrics),
            volatility_regime: format!("{:?}", metrics.ml_features.volatility_indicators.volatility_regime),
        }
    }
}

fn calculate_bollinger_signals(bands: &(Decimal, Decimal, Decimal)) -> Vec<f64> {
    vec![
        bands.0.to_f64().unwrap_or(0.0),  // Upper band
        bands.1.to_f64().unwrap_or(0.0),  // Middle band
        bands.2.to_f64().unwrap_or(0.0),  // Lower band
    ]
}

fn calculate_trend_strength(metrics: &EnhancedPoolMetrics) -> f64 {
    let returns = &metrics.base_metrics.performance_history.daily_returns;
    if returns.len() < 2 {
        return 0.0;
    }
    
    let mut positive_moves = 0;
    let mut total_moves = 0;
    
    for window in returns.windows(2) {
        if window[1].1 > window[0].1 {
            positive_moves += 1;
        }
        total_moves += 1;
    }
    
    if total_moves == 0 {
        0.0
    } else {
        positive_moves as f64 / total_moves as f64
    }
}

impl OptimizationFeatures {
    pub fn from(metrics: &EnhancedPoolMetrics) -> Self {
        Self {
            optimal_position_size: metrics.optimization_metrics.optimal_position_size.to_f64().unwrap_or(0.0),
            rebalance_signals: extract_rebalance_signals(&metrics.optimization_metrics.entry_signals),
            entry_points: extract_entry_points(&metrics.optimization_metrics.entry_signals),
            exit_points: extract_exit_points(&metrics.optimization_metrics.exit_signals),
            risk_adjusted_allocation: calculate_risk_adjusted_allocation(metrics),
            gas_optimization_score: calculate_gas_optimization_score(metrics),
            timing_efficiency: calculate_timing_efficiency(metrics),
        }
    }
}

fn extract_rebalance_signals(signals: &[Signal]) -> Vec<f64> {
    signals.iter()
        .filter(|s| matches!(s.signal_type, SignalType::Rebalance))
        .map(|s| s.strength.to_f64().unwrap_or(0.0))
        .collect()
}

fn calculate_timing_efficiency(metrics: &EnhancedPoolMetrics) -> f64 {
    let signals = &metrics.optimization_metrics.entry_signals;
    if signals.is_empty() {
        return 0.0;
    }
    
    let successful_signals = signals.iter()
        .filter(|s| s.confidence.to_f64().unwrap_or(0.0) > 0.8)
        .count();
        
    successful_signals as f64 / signals.len() as f64
} 