use super::*;
use near_sdk::test_utils::{accounts, VMContextBuilder};
use near_sdk::testing_env;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use std::str::FromStr;

mod analytics_tests {
    use super::*;

    fn setup_test_pool_metrics() -> PoolMetrics {
        PoolMetrics {
            pool_id: "test_pool".to_string(),
            pool_name: "TEST-USDC".to_string(),
            platform: "TestDex".to_string(),
            chain: "NEAR".to_string(),
            pool_type: PoolType::Weighted,
            creation_timestamp: 1677649200, // March 1, 2023

            tvl: 1_000_000_000_000_000_000_000_000, // 1M NEAR
            volume_24h: 100_000_000_000_000_000_000_000, // 100k NEAR
            volume_7d: 700_000_000_000_000_000_000_000, // 700k NEAR
            liquidity: 900_000_000_000_000_000_000_000, // 900k NEAR
            
            apy: APYBreakdown {
                total_apy: Decimal::from_str("0.15").unwrap(), // 15% APY
                base_apy: Decimal::from_str("0.10").unwrap(),
                reward_apy: Decimal::from_str("0.03").unwrap(),
                farming_apy: Decimal::from_str("0.02").unwrap(),
                historical_apy: vec![
                    (1677649200, Decimal::from_str("0.14").unwrap()),
                    (1677735600, Decimal::from_str("0.15").unwrap()),
                    (1677822000, Decimal::from_str("0.16").unwrap()),
                ],
                apy_stability_score: 85,
                projected_apy: Decimal::from_str("0.16").unwrap(),
            },

            impermanent_loss_risk: RiskScore {
                score: 35,
                risk_level: RiskLevel::Medium,
                historical_il: vec![
                    (1677649200, Decimal::from_str("-0.02").unwrap()),
                    (1677735600, Decimal::from_str("-0.015").unwrap()),
                ],
                max_il_projected: Decimal::from_str("-0.05").unwrap(),
                price_correlation: Decimal::from_str("0.75").unwrap(),
                risk_factors: vec![
                    RiskFactor {
                        factor_type: "volatility".to_string(),
                        impact_score: 40,
                        description: "Medium volatility impact".to_string(),
                    }
                ],
            },

            market_volatility: VolatilityMetrics {
                daily_volatility: Decimal::from_str("0.02").unwrap(),
                weekly_volatility: Decimal::from_str("0.05").unwrap(),
                monthly_volatility: Decimal::from_str("0.08").unwrap(),
                price_impact_1000usd: Decimal::from_str("0.001").unwrap(),
                price_impact_10000usd: Decimal::from_str("0.005").unwrap(),
                volatility_rank: 45,
                price_stability_score: 75,
            },

            security_score: SecurityMetrics {
                audit_score: 90,
                contract_risk: 20,
                centralization_risk: 30,
                exploit_history: vec![],
                insurance_coverage: true,
                security_features: vec!["timelock".to_string(), "emergency_shutdown".to_string()],
            },

            gas_metrics: MultiChainGasMetrics {
                near: GasMetrics {
                    avg_gas_cost: 100_000_000_000_000,
                    gas_token_price: Decimal::from_str("5.0").unwrap(),
                    cost_usd: Decimal::from_str("0.05").unwrap(),
                    gas_efficiency_score: 85,
                    peak_hours: vec![14, 15, 16],
                    historical_gas: vec![(1677649200, 95_000_000_000_000)],
                },
                aurora: GasMetrics {
                    avg_gas_cost: 150_000_000_000_000,
                    gas_token_price: Decimal::from_str("2000.0").unwrap(),
                    cost_usd: Decimal::from_str("0.08").unwrap(),
                    gas_efficiency_score: 80,
                    peak_hours: vec![13, 14, 15],
                    historical_gas: vec![(1677649200, 145_000_000_000_000)],
                },
                // ... other chains
            },

            fee_structure: FeeStructure {
                swap_fee: Decimal::from_str("0.003").unwrap(),
                protocol_fee: Decimal::from_str("0.001").unwrap(),
                lp_fee: Decimal::from_str("0.002").unwrap(),
                withdrawal_fee: Decimal::from_str("0.001").unwrap(),
                performance_fee: Decimal::from_str("0.10").unwrap(),
            },

            token_distribution: vec![
                TokenShare {
                    token_address: "token1.near".to_string(),
                    symbol: "TK1".to_string(),
                    weight: Decimal::from_str("0.5").unwrap(),
                    amount: 500_000_000_000_000_000_000_000,
                    value_usd: Decimal::from_str("500000.0").unwrap(),
                },
                TokenShare {
                    token_address: "token2.near".to_string(),
                    symbol: "TK2".to_string(),
                    weight: Decimal::from_str("0.5").unwrap(),
                    amount: 500_000_000_000_000_000_000_000,
                    value_usd: Decimal::from_str("500000.0").unwrap(),
                },
            ],

            user_metrics: UserMetrics {
                total_users: 1000,
                active_users_24h: 150,
                avg_position_size: 1_000_000_000_000_000_000_000,
                avg_holding_period: 604800, // 7 days
                user_concentration: Decimal::from_str("0.15").unwrap(),
            },

            performance_history: PerformanceHistory {
                daily_returns: vec![
                    (1677649200, Decimal::from_str("0.01").unwrap()),
                    (1677735600, Decimal::from_str("0.015").unwrap()),
                    (1677822000, Decimal::from_str("0.008").unwrap()),
                ],
                volume_history: vec![
                    (1677649200, 95_000_000_000_000_000_000_000),
                    (1677735600, 98_000_000_000_000_000_000_000),
                    (1677822000, 102_000_000_000_000_000_000_000),
                ],
                tvl_history: vec![
                    (1677649200, 980_000_000_000_000_000_000_000),
                    (1677735600, 990_000_000_000_000_000_000_000),
                    (1677822000, 1_000_000_000_000_000_000_000_000),
                ],
                il_history: vec![
                    (1677649200, Decimal::from_str("-0.02").unwrap()),
                    (1677735600, Decimal::from_str("-0.015").unwrap()),
                    (1677822000, Decimal::from_str("-0.01").unwrap()),
                ],
            },
        }
    }

    #[test]
    fn test_enhanced_pool_metrics() {
        let base_metrics = setup_test_pool_metrics();
        let enhanced_metrics = EnhancedPoolMetrics::new(base_metrics.clone());

        // Test advanced metrics
        assert!(enhanced_metrics.advanced_metrics.alpha_score >= Decimal::ZERO);
        assert!(enhanced_metrics.advanced_metrics.beta_coefficient > Decimal::ZERO);
        assert!(enhanced_metrics.advanced_metrics.sharpe_ratio >= Decimal::ZERO);
        assert!(enhanced_metrics.advanced_metrics.max_drawdown <= Decimal::ZERO);

        // Test ML features
        let momentum = &enhanced_metrics.ml_features.momentum_indicators;
        assert!(momentum.rsi_14 >= Decimal::ZERO && momentum.rsi_14 <= Decimal::from(100));
        assert!(momentum.momentum_score >= Decimal::ZERO);

        // Test optimization metrics
        assert!(enhanced_metrics.optimization_metrics.optimal_position_size > Decimal::ZERO);
        assert!(!enhanced_metrics.optimization_metrics.entry_signals.is_empty());
    }
}

mod ai_formatter_tests {
    use super::*;

    #[test]
    fn test_enhanced_ai_model_input() {
        let base_metrics = analytics_tests::setup_test_pool_metrics();
        let enhanced_metrics = EnhancedPoolMetrics::new(base_metrics);
        let ai_input = EnhancedAIModelInput::from(&enhanced_metrics);

        // Test pool features
        assert!(ai_input.pool_features.tvl_normalized >= 0.0 && ai_input.pool_features.tvl_normalized <= 1.0);
        assert!(ai_input.pool_features.capital_efficiency >= 0.0 && ai_input.pool_features.capital_efficiency <= 1.0);
        assert!(!ai_input.pool_features.chain_encoding.is_empty());

        // Test market features
        assert!(ai_input.market_features.price_volatility_1d >= 0.0);
        assert!(ai_input.market_features.liquidity_score >= 0.0);
        assert!(ai_input.market_features.depth_analysis.depth_2pct >= 0.0);

        // Test technical indicators
        assert!(!ai_input.technical_indicators.rsi_signals.is_empty());
        assert!(!ai_input.technical_indicators.macd_signals.is_empty());
        assert!(ai_input.technical_indicators.trend_strength >= 0.0 && ai_input.technical_indicators.trend_strength <= 1.0);

        // Test optimization features
        assert!(ai_input.optimization_features.optimal_position_size >= 0.0);
        assert!(ai_input.optimization_features.timing_efficiency >= 0.0 && ai_input.optimization_features.timing_efficiency <= 1.0);
    }
}

mod optimization_tests {
    use super::*;

    #[test]
    fn test_yield_optimization() {
        let base_metrics = analytics_tests::setup_test_pool_metrics();
        let enhanced_metrics = EnhancedPoolMetrics::new(base_metrics);

        // Test optimal position calculation
        let position_size = enhanced_metrics.optimization_metrics.optimal_position_size;
        assert!(position_size > Decimal::ZERO);
        assert!(position_size <= Decimal::from(enhanced_metrics.base_metrics.tvl));

        // Test entry signals
        let entry_signals = &enhanced_metrics.optimization_metrics.entry_signals;
        assert!(!entry_signals.is_empty());
        for signal in entry_signals {
            assert!(signal.confidence >= Decimal::ZERO && signal.confidence <= Decimal::ONE);
            assert!(signal.strength >= Decimal::ZERO);
        }

        // Test risk allocation
        let risk_allocation = &enhanced_metrics.optimization_metrics.risk_allocation;
        let total_allocation: Decimal = risk_allocation.values().sum();
        assert!(total_allocation <= Decimal::ONE + Decimal::new(1, 2)); // Allow for small rounding errors
    }
}

mod integration_tests {
    use super::*;

    #[test]
    fn test_end_to_end_analytics_pipeline() {
        // Setup test data
        let base_metrics = analytics_tests::setup_test_pool_metrics();
        
        // Test analytics pipeline
        let enhanced_metrics = EnhancedPoolMetrics::new(base_metrics.clone());
        let ai_input = EnhancedAIModelInput::from(&enhanced_metrics);

        // Verify analytics results
        assert!(enhanced_metrics.advanced_metrics.sharpe_ratio >= Decimal::ZERO);
        assert!(ai_input.performance_metrics.sharpe_ratio >= 0.0);

        // Verify optimization results
        assert!(!enhanced_metrics.optimization_metrics.entry_signals.is_empty());
        assert!(ai_input.optimization_features.optimal_position_size > 0.0);

        // Verify risk metrics
        assert!(enhanced_metrics.advanced_metrics.value_at_risk <= Decimal::ZERO);
        assert!(ai_input.performance_metrics.var_95 >= 0.0);
    }

    #[test]
    fn test_cross_chain_metrics() {
        let base_metrics = analytics_tests::setup_test_pool_metrics();
        let enhanced_metrics = EnhancedPoolMetrics::new(base_metrics);
        let ai_input = EnhancedAIModelInput::from(&enhanced_metrics);

        // Test cross-chain metrics
        let cross_chain = &ai_input.cross_chain_metrics;
        
        // Verify TVL shares
        assert!(!cross_chain.chain_tvl_share.is_empty());
        let total_tvl_share: f64 = cross_chain.chain_tvl_share.values().sum();
        assert!((total_tvl_share - 1.0).abs() < 0.01); // Should sum to approximately 1

        // Verify gas efficiency
        for (_, efficiency) in &cross_chain.bridge_efficiency {
            assert!(*efficiency >= 0.0 && *efficiency <= 1.0);
        }

        // Verify chain correlations
        assert!(!cross_chain.chain_correlation.is_empty());
        for row in &cross_chain.chain_correlation {
            for &corr in row {
                assert!(corr >= -1.0 && corr <= 1.0);
            }
        }
    }
}

// Helper function for test assertions
fn assert_decimal_range(value: Decimal, min: Decimal, max: Decimal) {
    assert!(value >= min && value <= max, 
        "Value {} not in range [{}, {}]", value, min, max);
}

fn assert_f64_range(value: f64, min: f64, max: f64) {
    assert!(value >= min && value <= max,
        "Value {} not in range [{}, {}]", value, min, max);
} 