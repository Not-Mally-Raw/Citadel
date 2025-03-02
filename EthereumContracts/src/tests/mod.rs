mod integration_tests;
mod mock_provider;

pub use mock_provider::MockProvider;

#[cfg(test)]
mod test_utils {
    use aurora_sdk::aurora_engine_types::{types::Address, U256};
    use std::str::FromStr;

    pub fn get_test_token_addresses() -> (Address, Address, Address) {
        (
            Address::from_str("0x6B175474E89094C44Da98b954EedeAC495271d0F").unwrap(), // DAI
            Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap(), // WETH
            Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap(), // USDC
        )
    }

    pub fn get_test_protocol_addresses() -> (Address, Address, Address) {
        (
            Address::from_str("0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9").unwrap(), // Aave lending pool
            Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D").unwrap(), // Uniswap router
            Address::from_str("0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7").unwrap(), // Curve 3pool
        )
    }

    pub fn get_test_amounts() -> (U256, U256, U256) {
        (
            U256::from(1_000_000_000_000_000_000u64),     // 1 ETH/token
            U256::from(1_000_000u64),                      // 1 USDC (6 decimals)
            U256::from(1_000_000_000_000_000_000_000u64), // 1000 tokens
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AuroraIntegration, CrossChainConfig, ProtocolConfig, ProtocolType,
        protocols::{AaveProtocol, UniswapProtocol, CurveProtocol},
        bridge::RainbowBridge,
    };
    use test_utils::*;
    use tokio_test::block_on;

    #[test]
    fn test_all_features() {
        block_on(async {
            // Set up mock provider
            let mock_provider = MockProvider::new();
            let (dai, weth, usdc) = get_test_token_addresses();
            let (amount_eth, amount_usdc, amount_large) = get_test_amounts();

            // Test deposit and withdraw
            mock_provider.set_balance(dai, amount_large).await;
            
            let deposit_result = mock_provider.deposit(dai, amount_eth).await;
            assert!(deposit_result.is_ok());
            
            let withdraw_result = mock_provider.withdraw(dai, amount_eth).await;
            assert!(withdraw_result.is_ok());

            // Test protocol interactions
            let (lending_pool, router, pool) = get_test_protocol_addresses();
            
            // Aave test
            let aave = AaveProtocol::new(lending_pool, "https://testnet.aurora.dev");
            let aave_deposit = aave.deposit(dai, amount_eth).await;
            assert!(aave_deposit.is_err()); // Should fail since we're not on mainnet
            
            // Uniswap test
            let uniswap = UniswapProtocol::new(router, router, "https://testnet.aurora.dev");
            let swap_result = uniswap.swap(crate::protocols::SwapParams {
                token_in: dai,
                token_out: weth,
                amount_in: amount_eth,
                min_amount_out: U256::zero(),
                deadline: 0,
            }).await;
            assert!(swap_result.is_err()); // Should fail since we're not on mainnet
            
            // Curve test
            let curve = CurveProtocol::new(pool, pool, "https://testnet.aurora.dev");
            let curve_exchange = curve.exchange(0, 1, amount_eth, U256::zero()).await;
            assert!(curve_exchange.is_err()); // Should fail since we're not on mainnet

            // Test metrics tracking
            let tx_count = mock_provider.get_transaction_count().await;
            assert_eq!(tx_count, 2); // One deposit and one withdrawal
        });
    }
} 