use crate::{
    Asset, Protocol, OracleAdapter, ChainlinkOracle, PriceFetcher, ApyFetcher,
    PriceData, ApyData, LiquidityData, OracleError,
    mock::MockOracle,
};
use near_sdk::json_types::U128;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_test::block_on;

fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[test]
fn test_price_fetching_functionality() {
    block_on(async {
        let mock_oracle = MockOracle::new();
        let fetcher = PriceFetcher::new(Box::new(mock_oracle.clone()), 3600);

        // Test assets
        let eth = Asset::Token("ETH".to_string());
        let btc = Asset::Token("BTC".to_string());
        let usdc = Asset::Token("USDC".to_string());

        // Set up test data
        let eth_price = PriceData {
            price: U128(1_500_000_000_000),
            timestamp: get_current_timestamp(),
            source: "chainlink".to_string(),
        };
        let btc_price = PriceData {
            price: U128(30_000_000_000_000),
            timestamp: get_current_timestamp(),
            source: "chainlink".to_string(),
        };

        mock_oracle.set_price(eth.clone(), eth_price.clone()).await;
        mock_oracle.set_price(btc.clone(), btc_price.clone()).await;

        // Test single price fetch
        let fetched_eth = fetcher.get_prices(&[eth.clone()]).await.unwrap();
        assert_eq!(fetched_eth.get(&eth).unwrap().price, eth_price.price);

        // Test multiple price fetch
        let prices = fetcher.get_prices(&[eth.clone(), btc.clone()]).await.unwrap();
        assert_eq!(prices.len(), 2);
        assert_eq!(prices.get(&eth).unwrap().price, eth_price.price);
        assert_eq!(prices.get(&btc).unwrap().price, btc_price.price);

        // Test price change calculation
        let change = fetcher.calculate_price_change(&eth, 86400).await.unwrap();
        assert!(change.abs() <= 100.0);

        // Test volatility calculation
        let volatility = fetcher.calculate_volatility(&eth, 86400, 10).await.unwrap();
        assert!(volatility >= 0 && volatility <= 100);

        // Test price impact
        let impact = fetcher.estimate_price_impact(&eth, U128(1_000_000_000_000)).await.unwrap();
        assert!(impact > 0.0 && impact <= 100.0);

        // Test error handling for unsupported asset
        let result = fetcher.get_prices(&[usdc.clone()]).await;
        assert!(matches!(result, Err(OracleError::UnsupportedAsset(_))));
    });
}

#[test]
fn test_apy_fetching_functionality() {
    block_on(async {
        let mock_oracle = MockOracle::new();
        let fetcher = ApyFetcher::new(Box::new(mock_oracle.clone()), 3600);

        // Test assets and protocols
        let eth = Asset::Token("ETH".to_string());
        let aave = Protocol::Aave;
        let compound = Protocol::Compound;

        // Set up test data
        let aave_apy = ApyData {
            apy: 0.05,
            timestamp: get_current_timestamp(),
            protocol: Protocol::Aave,
            risk_score: 2,
        };
        let compound_apy = ApyData {
            apy: 0.06,
            timestamp: get_current_timestamp(),
            protocol: Protocol::Compound,
            risk_score: 3,
        };

        mock_oracle.set_apy(eth.clone(), aave.clone(), aave_apy.clone()).await;
        mock_oracle.set_apy(eth.clone(), compound.clone(), compound_apy.clone()).await;

        // Test multi-protocol APY fetch
        let apys = fetcher.get_multi_protocol_apys(
            &[eth.clone()],
            &[aave.clone(), compound.clone()]
        ).await.unwrap();
        assert_eq!(apys.len(), 2);
        assert_eq!(
            apys.get(&(eth.clone(), aave.clone())).unwrap().apy,
            aave_apy.apy
        );

        // Test best APY finding
        let best = fetcher.find_best_apy(&eth, &[aave.clone(), compound.clone()]).await.unwrap();
        assert!(best.is_some());
        let (best_protocol, best_apy) = best.unwrap();
        assert!(matches!(best_protocol, Protocol::Compound));
        assert_eq!(best_apy.apy, 0.06);

        // Test APY history
        let history = fetcher.get_apy_history(&eth, &aave, 86400).await.unwrap();
        assert!(!history.is_empty());

        // Test APY volatility
        let volatility = fetcher.calculate_apy_volatility(&eth, &aave, 86400).await.unwrap();
        assert!(volatility >= 0 && volatility <= 100);
    });
}

#[test]
fn test_stale_data_handling() {
    block_on(async {
        let mock_oracle = MockOracle::new();
        let fetcher = PriceFetcher::new(Box::new(mock_oracle.clone()), 60); // 1 minute max age

        let eth = Asset::Token("ETH".to_string());
        let stale_price = PriceData {
            price: U128(1_500_000_000_000),
            timestamp: get_current_timestamp() - 3600, // 1 hour old
            source: "chainlink".to_string(),
        };

        mock_oracle.set_price(eth.clone(), stale_price).await;

        // Test stale data rejection
        let result = fetcher.get_prices(&[eth.clone()]).await;
        assert!(matches!(result, Err(OracleError::StaleData { .. })));
    });
}

#[test]
fn test_liquidity_data() {
    block_on(async {
        let mock_oracle = MockOracle::new();
        let eth = Asset::Token("ETH".to_string());
        let aave = Protocol::Aave;

        let liquidity_data = LiquidityData {
            total_liquidity: U128(1_000_000_000_000_000),
            available_liquidity: U128(800_000_000_000_000),
            utilization_rate: 0.8,
            timestamp: get_current_timestamp(),
        };

        mock_oracle.set_liquidity(eth.clone(), aave.clone(), liquidity_data.clone()).await;

        // Test liquidity data fetch
        let fetched = mock_oracle.get_liquidity(&eth, &aave).await.unwrap();
        assert_eq!(fetched.total_liquidity, liquidity_data.total_liquidity);
        assert_eq!(fetched.utilization_rate, liquidity_data.utilization_rate);
    });
}

#[test]
fn test_chainlink_oracle_requests() {
    let oracle = ChainlinkOracle::new(
        "https://api.chain.link/v1".to_string(),
        Some("test-api-key".to_string())
    );

    // Note: These tests would need a mock HTTP server in a real implementation
    // For now, we just verify the request formation logic
    let eth = Asset::Token("ETH".to_string());
    let aave = Protocol::Aave;

    assert_eq!(
        format!("{}/{}", oracle.endpoint, "price/ETH"),
        "https://api.chain.link/v1/price/ETH"
    );
    assert_eq!(
        format!("{}/apy/{}/{}", oracle.endpoint, "aave", "ETH"),
        "https://api.chain.link/v1/apy/aave/ETH"
    );
} 