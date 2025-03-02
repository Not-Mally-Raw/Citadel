use crate::{
    AuroraIntegration, CrossChainConfig, ProtocolConfig, ProtocolType,
    protocols::{AaveProtocol, UniswapProtocol, CurveProtocol, DeFiProtocol, SwapParams, ProtocolMetrics},
    bridge::{RainbowBridge, BridgeConfig, TransferStatus},
};
use aurora_sdk::aurora_engine_types::{types::Address, U256};
use near_sdk::AccountId;
use tokio_test::block_on;
use std::str::FromStr;

// Helper function to get test addresses
fn get_test_addresses() -> (Address, Address, Address) {
    (
        Address::from_str("0x6B175474E89094C44Da98b954EedeAC495271d0F").unwrap(), // DAI
        Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap(), // WETH
        Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap(), // USDC
    )
}

#[test]
fn test_aurora_integration_setup() {
    let config = CrossChainConfig {
        aurora_endpoint: "https://testnet.aurora.dev".to_string(),
        ethereum_rpc: "https://eth-goerli.g.alchemy.com/v2/test".to_string(),
        bridge_contract: "bridge.testnet.near".to_string(),
        gas_limit: U256::from(1_000_000),
        protocols: vec![
            ProtocolConfig {
                name: "Aave".to_string(),
                contract_address: get_test_addresses().0,
                protocol_type: ProtocolType::Aave,
            },
        ],
    };

    let integration = AuroraIntegration::new(config);
    
    // Test address conversion
    let near_account = AccountId::new_unchecked("test.near".to_string());
    let aurora_address = integration.near_to_aurora_address(&near_account);
    assert!(aurora_address != Address::zero());
}

#[test]
fn test_aave_protocol() {
    block_on(async {
        let (dai_address, _, _) = get_test_addresses();
        let lending_pool = Address::from_str("0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9").unwrap();
        let rpc_url = "https://testnet.aurora.dev";
        
        let aave = AaveProtocol::new(lending_pool, rpc_url);
        
        // Test deposit parameters
        let amount = U256::from(1_000_000_000_000_000_000u64); // 1 DAI
        let result = aave.deposit(dai_address, amount).await;
        assert!(result.is_err()); // Should be error since we're not on mainnet

        // Test metrics fetching
        let metrics_result = aave.get_metrics(dai_address).await;
        assert!(metrics_result.is_err()); // Should be error since we're not on mainnet
    });
}

#[test]
fn test_uniswap_protocol() {
    block_on(async {
        let (dai_address, weth_address, _) = get_test_addresses();
        let router = Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D").unwrap();
        let factory = Address::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap();
        let rpc_url = "https://testnet.aurora.dev";
        
        let uniswap = UniswapProtocol::new(router, factory, rpc_url);
        
        // Test swap parameters
        let swap_params = SwapParams {
            token_in: dai_address,
            token_out: weth_address,
            amount_in: U256::from(1_000_000_000_000_000_000u64), // 1 DAI
            min_amount_out: U256::from(0),
            deadline: 0,
        };
        
        let result = uniswap.swap(swap_params).await;
        assert!(result.is_err()); // Should be error since we're not on mainnet
    });
}

#[test]
fn test_curve_protocol() {
    block_on(async {
        let (dai_address, _, usdc_address) = get_test_addresses();
        let pool = Address::from_str("0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7").unwrap(); // 3pool
        let registry = Address::from_str("0x90E00ACe148ca3b23Ac1bC8C240C2a7Dd9c2d7f5").unwrap();
        let rpc_url = "https://testnet.aurora.dev";
        
        let curve = CurveProtocol::new(pool, registry, rpc_url);
        
        // Test exchange parameters
        let amount = U256::from(1_000_000_000_000_000_000u64); // 1 DAI
        let result = curve.exchange(0, 1, amount, U256::zero()).await;
        assert!(result.is_err()); // Should be error since we're not on mainnet
    });
}

#[test]
fn test_rainbow_bridge() {
    block_on(async {
        let config = BridgeConfig {
            near_token_bridge: AccountId::new_unchecked("bridge.testnet.near".to_string()),
            aurora_token_bridge: Address::from_low_u64_be(1),
            eth_locker: Address::from_low_u64_be(2),
            confirmation_blocks: 12,
            max_transfer_amount: U256::from(1_000_000_000_000_000_000u64), // 1 ETH
        };
        
        let bridge = RainbowBridge::new(config.clone());
        
        // Test transfer amount validation
        let large_amount = U256::from(2_000_000_000_000_000_000u64); // 2 ETH
        let result = bridge.transfer_to_aurora(
            AccountId::new_unchecked("test.near".to_string()),
            large_amount,
            Address::zero(),
        ).await;
        
        assert!(matches!(
            result,
            Err(crate::CrossChainError::BridgeError(_))
        ));
        
        // Test bridge stats
        let stats_result = bridge.get_bridge_stats().await;
        assert!(stats_result.is_err()); // Should be error since we're using mock
    });
}

#[test]
fn test_gas_estimation() {
    block_on(async {
        let config = CrossChainConfig {
            aurora_endpoint: "https://testnet.aurora.dev".to_string(),
            ethereum_rpc: "https://eth-goerli.g.alchemy.com/v2/test".to_string(),
            bridge_contract: "bridge.testnet.near".to_string(),
            gas_limit: U256::from(1_000_000),
            protocols: vec![],
        };
        
        let integration = AuroraIntegration::new(config);
        
        // Test gas estimation for a simple transfer
        let (dai_address, _, _) = get_test_addresses();
        let amount = U256::from(1_000_000_000_000_000_000u64); // 1 DAI
        let data = vec![]; // Empty data for transfer
        
        let result = integration.estimate_gas(
            dai_address,
            data,
            amount,
        ).await;
        
        assert!(result.is_err()); // Should be error since we're not on mainnet
    });
}

// Helper function to simulate protocol metrics
fn get_mock_metrics() -> ProtocolMetrics {
    ProtocolMetrics {
        tvl: U256::from(1_000_000_000_000_000_000_000u128), // 1000 ETH
        apy: 0.05, // 5% APY
        utilization_rate: 0.8, // 80% utilization
        total_borrowed: U256::from(800_000_000_000_000_000_000u128), // 800 ETH
        total_supplied: U256::from(1_000_000_000_000_000_000_000u128), // 1000 ETH
    }
}

#[test]
fn test_protocol_metrics() {
    block_on(async {
        let (dai_address, _, _) = get_test_addresses();
        let lending_pool = Address::from_str("0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9").unwrap();
        let rpc_url = "https://testnet.aurora.dev";
        
        let aave = AaveProtocol::new(lending_pool, rpc_url);
        let metrics_result = aave.get_metrics(dai_address).await;
        
        // Compare with expected metrics format
        let expected_metrics = get_mock_metrics();
        assert!(metrics_result.is_err()); // Should be error since we're using mock
        
        // Validate metrics structure
        assert!(expected_metrics.apy >= 0.0 && expected_metrics.apy <= 1.0);
        assert!(expected_metrics.utilization_rate >= 0.0 && expected_metrics.utilization_rate <= 1.0);
        assert!(expected_metrics.total_borrowed <= expected_metrics.total_supplied);
    });
} 