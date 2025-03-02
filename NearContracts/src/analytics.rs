use near_sdk::{Balance, AccountId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PoolMetrics {
    // Basic Pool Info
    pub pool_id: String,
    pub pool_name: String,
    pub platform: String,
    pub chain: String,
    pub pool_type: PoolType,
    pub creation_timestamp: u64,

    // Core Metrics
    pub tvl: Balance,
    pub volume_24h: Balance,
    pub volume_7d: Balance,
    pub liquidity: Balance,
    pub apy: APYBreakdown,
    
    // Risk Metrics
    pub impermanent_loss_risk: RiskScore,
    pub market_volatility: VolatilityMetrics,
    pub security_score: SecurityMetrics,
    
    // Gas & Cost Analysis
    pub gas_metrics: MultiChainGasMetrics,
    pub fee_structure: FeeStructure,
    
    // Additional Analytics
    pub token_distribution: Vec<TokenShare>,
    pub user_metrics: UserMetrics,
    pub performance_history: PerformanceHistory,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PoolType {
    Stable,
    Volatile,
    Weighted,
    Concentrated,
    Hybrid,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct APYBreakdown {
    pub total_apy: Decimal,
    pub base_apy: Decimal,
    pub reward_apy: Decimal,
    pub farming_apy: Decimal,
    pub historical_apy: Vec<(u64, Decimal)>,
    pub apy_stability_score: u32,
    pub projected_apy: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RiskScore {
    pub score: u32,  // 0-100
    pub risk_level: RiskLevel,
    pub historical_il: Vec<(u64, Decimal)>,
    pub max_il_projected: Decimal,
    pub price_correlation: Decimal,
    pub risk_factors: Vec<RiskFactor>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RiskLevel {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RiskFactor {
    pub factor_type: String,
    pub impact_score: u32,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VolatilityMetrics {
    pub daily_volatility: Decimal,
    pub weekly_volatility: Decimal,
    pub monthly_volatility: Decimal,
    pub price_impact_1000usd: Decimal,
    pub price_impact_10000usd: Decimal,
    pub volatility_rank: u32,  // 1-100
    pub price_stability_score: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityMetrics {
    pub audit_score: u32,
    pub contract_risk: u32,
    pub centralization_risk: u32,
    pub exploit_history: Vec<SecurityEvent>,
    pub insurance_coverage: bool,
    pub security_features: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityEvent {
    pub timestamp: u64,
    pub event_type: String,
    pub severity: String,
    pub description: String,
    pub resolution: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MultiChainGasMetrics {
    pub near: GasMetrics,
    pub aurora: GasMetrics,
    pub bsc: GasMetrics,
    pub polygon: GasMetrics,
    pub avalanche: GasMetrics,
    pub solana: GasMetrics,
    pub arbitrum: GasMetrics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GasMetrics {
    pub avg_gas_cost: Balance,
    pub gas_token_price: Decimal,
    pub cost_usd: Decimal,
    pub gas_efficiency_score: u32,
    pub peak_hours: Vec<u32>,
    pub historical_gas: Vec<(u64, Balance)>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeeStructure {
    pub swap_fee: Decimal,
    pub protocol_fee: Decimal,
    pub lp_fee: Decimal,
    pub withdrawal_fee: Decimal,
    pub performance_fee: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenShare {
    pub token_address: String,
    pub symbol: String,
    pub weight: Decimal,
    pub amount: Balance,
    pub value_usd: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserMetrics {
    pub total_users: u32,
    pub active_users_24h: u32,
    pub avg_position_size: Balance,
    pub avg_holding_period: u64,
    pub user_concentration: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerformanceHistory {
    pub daily_returns: Vec<(u64, Decimal)>,
    pub volume_history: Vec<(u64, Balance)>,
    pub tvl_history: Vec<(u64, Balance)>,
    pub il_history: Vec<(u64, Decimal)>,
}

// Analytics Implementation
impl PoolMetrics {
    pub fn calculate_risk_adjusted_apy(&self) -> Decimal {
        let base = self.apy.total_apy;
        let risk_multiplier = Decimal::from(100 - self.impermanent_loss_risk.score) / Decimal::from(100);
        base * risk_multiplier
    }

    pub fn estimate_impermanent_loss(&self, price_change_pct: Decimal) -> Decimal {
        // IL = 2√(P₁/P₀) / (1 + P₁/P₀) - 1
        let price_ratio = Decimal::ONE + price_change_pct;
        let sqrt_ratio = price_ratio.sqrt().unwrap_or(Decimal::ONE);
        (Decimal::TWO * sqrt_ratio / (Decimal::ONE + price_ratio)) - Decimal::ONE
    }

    pub fn get_optimal_entry_exit(&self) -> (String, String) {
        let gas_metrics = &self.gas_metrics;
        let best_entry = gas_metrics.near.peak_hours
            .iter()
            .min()
            .map(|h| format!("{}:00 UTC", h))
            .unwrap_or_default();
        
        let worst_entry = gas_metrics.near.peak_hours
            .iter()
            .max()
            .map(|h| format!("{}:00 UTC", h))
            .unwrap_or_default();

        (best_entry, worst_entry)
    }

    pub fn calculate_volatility_impact(&self, amount_usd: Decimal) -> Decimal {
        let base_impact = if amount_usd <= Decimal::from(1000) {
            self.volatility_metrics.price_impact_1000usd
        } else {
            self.volatility_metrics.price_impact_10000usd
        };

        base_impact * (amount_usd / Decimal::from(1000)).sqrt().unwrap_or(Decimal::ONE)
    }
}

// Helper functions for analytics
pub fn calculate_correlation(x: &[Decimal], y: &[Decimal]) -> Decimal {
    if x.len() != y.len() || x.is_empty() {
        return Decimal::ZERO;
    }

    let n = Decimal::from(x.len());
    let sum_x: Decimal = x.iter().sum();
    let sum_y: Decimal = y.iter().sum();
    let sum_xy: Decimal = x.iter().zip(y.iter()).map(|(a, b)| *a * *b).sum();
    let sum_x_sq: Decimal = x.iter().map(|a| *a * *a).sum();
    let sum_y_sq: Decimal = y.iter().map(|b| *b * *b).sum();

    let numerator = n * sum_xy - sum_x * sum_y;
    let denominator = ((n * sum_x_sq - sum_x * sum_x) * (n * sum_y_sq - sum_y * sum_y)).sqrt().unwrap_or(Decimal::ONE);

    if denominator == Decimal::ZERO {
        Decimal::ZERO
    } else {
        numerator / denominator
    }
}

pub fn calculate_volatility(prices: &[(u64, Decimal)], window: u64) -> Decimal {
    if prices.len() < 2 {
        return Decimal::ZERO;
    }

    let returns: Vec<Decimal> = prices.windows(2)
        .map(|w| (w[1].1 - w[0].1) / w[0].1)
        .collect();

    let mean = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());
    let variance = returns.iter()
        .map(|r| (*r - mean) * (*r - mean))
        .sum::<Decimal>() / Decimal::from(returns.len() - 1);

    variance.sqrt().unwrap_or(Decimal::ZERO)
}

// Advanced Analytics Enhancements
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnhancedPoolMetrics {
    pub base_metrics: PoolMetrics,
    pub advanced_metrics: AdvancedMetrics,
    pub ml_features: MLFeatures,
    pub optimization_metrics: OptimizationMetrics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdvancedMetrics {
    pub alpha_score: Decimal,          // Strategy's excess return compared to benchmark
    pub beta_coefficient: Decimal,     // Systematic risk measure
    pub sharpe_ratio: Decimal,         // Risk-adjusted return metric
    pub sortino_ratio: Decimal,       // Downside risk-adjusted return
    pub max_drawdown: Decimal,        // Largest peak-to-trough decline
    pub value_at_risk: Decimal,       // Statistical measure of potential loss
    pub calmar_ratio: Decimal,        // Return/Max Drawdown ratio
    pub omega_ratio: Decimal,         // Probability-weighted ratio of gains vs losses
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MLFeatures {
    pub momentum_indicators: MomentumIndicators,
    pub volatility_indicators: VolatilityIndicators,
    pub market_indicators: MarketIndicators,
    pub sentiment_metrics: SentimentMetrics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MomentumIndicators {
    pub rsi_14: Decimal,              // Relative Strength Index
    pub macd: (Decimal, Decimal),     // Moving Average Convergence Divergence
    pub rate_of_change: Decimal,      // Price Rate of Change
    pub momentum_score: Decimal,      // Composite momentum score
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VolatilityIndicators {
    pub bollinger_bands: (Decimal, Decimal, Decimal),  // Upper, Middle, Lower
    pub average_true_range: Decimal,                   // ATR
    pub historical_volatility: Vec<(u64, Decimal)>,    // Time series of volatility
    pub volatility_regime: VolatilityRegime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketIndicators {
    pub market_depth: Decimal,
    pub bid_ask_spread: Decimal,
    pub liquidity_score: Decimal,
    pub market_impact: Decimal,
    pub market_efficiency_coefficient: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SentimentMetrics {
    pub social_volume: u64,
    pub sentiment_score: Decimal,
    pub developer_activity: u32,
    pub governance_participation: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptimizationMetrics {
    pub optimal_position_size: Decimal,
    pub rebalancing_threshold: Decimal,
    pub entry_signals: Vec<Signal>,
    pub exit_signals: Vec<Signal>,
    pub risk_allocation: HashMap<String, Decimal>,
    pub opportunity_score: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VolatilityRegime {
    Low,
    Medium,
    High,
    Extreme,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Signal {
    pub timestamp: u64,
    pub signal_type: SignalType,
    pub strength: Decimal,
    pub confidence: Decimal,
    pub indicators: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SignalType {
    Entry,
    Exit,
    Rebalance,
    RiskWarning,
}

// Enhanced Implementation
impl EnhancedPoolMetrics {
    pub fn new(base_metrics: PoolMetrics) -> Self {
        Self {
            base_metrics: base_metrics.clone(),
            advanced_metrics: Self::calculate_advanced_metrics(&base_metrics),
            ml_features: Self::generate_ml_features(&base_metrics),
            optimization_metrics: Self::compute_optimization_metrics(&base_metrics),
        }
    }

    fn calculate_advanced_metrics(metrics: &PoolMetrics) -> AdvancedMetrics {
        let returns = Self::calculate_returns(&metrics.performance_history.daily_returns);
        let volatility = Self::calculate_volatility(&returns);
        let downside_returns = Self::calculate_downside_returns(&returns);
        
        AdvancedMetrics {
            alpha_score: Self::calculate_alpha(&returns),
            beta_coefficient: Self::calculate_beta(&returns),
            sharpe_ratio: Self::calculate_sharpe_ratio(&returns, &volatility),
            sortino_ratio: Self::calculate_sortino_ratio(&returns, &downside_returns),
            max_drawdown: Self::calculate_max_drawdown(&metrics.performance_history.tvl_history),
            value_at_risk: Self::calculate_var(&returns, Decimal::new(95, 2)), // 95% confidence
            calmar_ratio: Self::calculate_calmar_ratio(&returns),
            omega_ratio: Self::calculate_omega_ratio(&returns),
        }
    }

    fn generate_ml_features(metrics: &PoolMetrics) -> MLFeatures {
        MLFeatures {
            momentum_indicators: Self::calculate_momentum_indicators(metrics),
            volatility_indicators: Self::calculate_volatility_indicators(metrics),
            market_indicators: Self::calculate_market_indicators(metrics),
            sentiment_metrics: Self::calculate_sentiment_metrics(metrics),
        }
    }

    fn compute_optimization_metrics(metrics: &PoolMetrics) -> OptimizationMetrics {
        OptimizationMetrics {
            optimal_position_size: Self::calculate_optimal_position(metrics),
            rebalancing_threshold: Self::calculate_rebalancing_threshold(metrics),
            entry_signals: Self::generate_entry_signals(metrics),
            exit_signals: Self::generate_exit_signals(metrics),
            risk_allocation: Self::calculate_risk_allocation(metrics),
            opportunity_score: Self::calculate_opportunity_score(metrics),
        }
    }

    // Advanced calculation methods
    fn calculate_alpha(returns: &[Decimal]) -> Decimal {
        if returns.is_empty() {
            return Decimal::ZERO;
        }
        
        let market_return = Decimal::new(8, 2); // Assumed market return of 8%
        let risk_free_rate = Decimal::new(2, 2); // Assumed risk-free rate of 2%
        
        let avg_return: Decimal = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());
        avg_return - (risk_free_rate + market_return)
    }

    fn calculate_beta(returns: &[Decimal]) -> Decimal {
        if returns.is_empty() {
            return Decimal::ONE;
        }

        let market_returns = vec![Decimal::new(1, 2); returns.len()]; // Simplified market returns
        let covariance = Self::calculate_covariance(returns, &market_returns);
        let market_variance = Self::calculate_variance(&market_returns);
        
        if market_variance == Decimal::ZERO {
            Decimal::ONE
        } else {
            covariance / market_variance
        }
    }

    fn calculate_optimal_position(metrics: &PoolMetrics) -> Decimal {
        let tvl = Decimal::from(metrics.tvl);
        let volatility = metrics.market_volatility.daily_volatility;
        let risk_score = Decimal::from(metrics.impermanent_loss_risk.score);
        
        // Kelly Criterion based position sizing
        let win_ratio = Decimal::ONE - (risk_score / Decimal::from(100));
        let loss_ratio = risk_score / Decimal::from(100);
        
        if loss_ratio >= Decimal::ONE {
            return Decimal::ZERO;
        }
        
        let kelly_fraction = (win_ratio / loss_ratio) - Decimal::ONE;
        let position_size = (tvl * kelly_fraction) / Decimal::from(2);
        
        // Apply volatility adjustment
        position_size * (Decimal::ONE - volatility)
    }

    fn generate_entry_signals(metrics: &PoolMetrics) -> Vec<Signal> {
        let mut signals = Vec::new();
        let price_history = &metrics.performance_history.daily_returns;
        
        if price_history.len() < 2 {
            return signals;
        }

        // Generate signals based on multiple indicators
        for window in price_history.windows(2) {
            let (prev_timestamp, prev_return) = window[0];
            let (curr_timestamp, curr_return) = window[1];
            
            // Momentum signal
            if curr_return > prev_return && curr_return > Decimal::new(5, 2) {
                signals.push(Signal {
                    timestamp: curr_timestamp,
                    signal_type: SignalType::Entry,
                    strength: curr_return / Decimal::new(5, 2),
                    confidence: Decimal::new(85, 2),
                    indicators: vec!["momentum".to_string(), "trend_following".to_string()],
                });
            }
            
            // Value signal
            if curr_return < Decimal::ZERO && metrics.market_volatility.price_stability_score > 70 {
                signals.push(Signal {
                    timestamp: curr_timestamp,
                    signal_type: SignalType::Entry,
                    strength: Decimal::ONE - curr_return.abs(),
                    confidence: Decimal::new(75, 2),
                    indicators: vec!["mean_reversion".to_string(), "value".to_string()],
                });
            }
        }

        signals
    }

    // Helper methods for statistical calculations
    fn calculate_covariance(x: &[Decimal], y: &[Decimal]) -> Decimal {
        if x.len() != y.len() || x.is_empty() {
            return Decimal::ZERO;
        }

        let mean_x = x.iter().sum::<Decimal>() / Decimal::from(x.len());
        let mean_y = y.iter().sum::<Decimal>() / Decimal::from(y.len());
        
        let sum_cov = x.iter().zip(y.iter())
            .map(|(xi, yi)| (*xi - mean_x) * (*yi - mean_y))
            .sum::<Decimal>();
            
        sum_cov / Decimal::from(x.len() - 1)
    }

    fn calculate_variance(x: &[Decimal]) -> Decimal {
        if x.is_empty() {
            return Decimal::ZERO;
        }

        let mean = x.iter().sum::<Decimal>() / Decimal::from(x.len());
        let sum_squared_diff = x.iter()
            .map(|xi| (*xi - mean) * (*xi - mean))
            .sum::<Decimal>();
            
        sum_squared_diff / Decimal::from(x.len() - 1)
    }
} 