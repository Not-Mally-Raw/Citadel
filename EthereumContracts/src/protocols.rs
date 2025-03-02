use ethers::{
    types::{Address, U256},
    providers::{Provider, Http},
};
use std::sync::Arc;
use web3::types::Transaction;
use crate::{CrossChainError, TransactionRequest, ProtocolType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ethers::abi::Tokenizable;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolMetrics {
    pub tvl: EthersU256,
    pub apy: f64,
    pub utilization_rate: f64,
    pub total_borrowed: EthersU256,
    pub total_supplied: EthersU256,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapParams {
    pub token_in: EthersAddress,
    pub token_out: EthersAddress,
    pub amount_in: EthersU256,
    pub min_amount_out: EthersU256,
    pub deadline: u64,
}

#[async_trait(?Send)]
pub trait DeFiProtocol {
    async fn deposit(&self, token: EthersAddress, amount: EthersU256) -> Result<Transaction, CrossChainError>;
    async fn withdraw(&self, token: EthersAddress, amount: EthersU256) -> Result<Transaction, CrossChainError>;
    async fn get_metrics(&self, token: EthersAddress) -> Result<ProtocolMetrics, CrossChainError>;
}

pub struct UniswapProtocol {
    provider: Provider<Http>,
    router_address: EthersAddress,
}

impl UniswapProtocol {
    pub fn new(rpc_url: &str, router_address: EthersAddress) -> Result<Self, CrossChainError> {
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| CrossChainError::ProviderError(e.to_string()))?;
        
        Ok(Self {
            provider,
            router_address,
        })
    }

    pub async fn swap(
        &self,
        token_in: EthersAddress,
        token_out: EthersAddress,
        amount_in: EthersU256,
        min_amount_out: EthersU256,
    ) -> Result<Transaction, CrossChainError> {
        let function_selector = [0x7c, 0x02, 0x5, 0x20]; // swapExactTokensForTokens selector
        let mut data = Vec::with_capacity(4 + 32 * 4);
        data.extend_from_slice(&function_selector);
        data.extend_from_slice(&ethers::abi::encode(&[
            token_in.into_token(),
            token_out.into_token(),
            amount_in.into_token(),
            min_amount_out.into_token()
        ]));

        let request = TransactionRequest::new()
            .to(self.router_address)
            .data(data)
            .gas_limit(Web3U256::from(300000));

        crate::send_transaction(&self.provider, request).await
    }
}

#[async_trait(?Send)]
impl DeFiProtocol for UniswapProtocol {
    async fn deposit(&self, token: EthersAddress, amount: EthersU256) -> Result<Transaction, CrossChainError> {
        let function_selector = [0xe8, 0xe3, 0x3d, 0x8e]; // deposit selector
        let mut data = Vec::with_capacity(4 + 32 * 2);
        data.extend_from_slice(&function_selector);
        data.extend_from_slice(&ethers::abi::encode(&[
            token.into_token(),
            amount.into_token()
        ]));

        let request = TransactionRequest::new()
            .to(self.router_address)
            .data(data)
            .gas_limit(Web3U256::from(200000));

        crate::send_transaction(&self.provider, request).await
    }

    async fn withdraw(&self, token: Address, amount: U256) -> Result<Transaction, CrossChainError> {
        let function_selector = [0x2e, 0x1a, 0x7d, 0x4d]; // withdraw selector
        let mut data = Vec::with_capacity(4 + 32 * 2);
        data.extend_from_slice(&function_selector);
        data.extend_from_slice(&ethers::abi::encode(&[
            token.into_token(),
            amount.into_token()
        ]));

        let request = TransactionRequest::new()
            .to(self.router_address)
            .value(Web3U256::zero())
            .data(data)
            .gas_limit(Web3U256::from(200000));

        crate::send_transaction(&self.provider, request).await
    }

    async fn get_metrics(&self, token: Address) -> Result<ProtocolMetrics, CrossChainError> {
        let contract = Contract::new(
            self.router_address,
            include_bytes!("../abis/uniswap_v2.json"),
            Arc::new(self.provider.clone())
        );

        let (tvl, apy_raw, utilization_raw, total_borrowed, total_supplied): (U256, U256, U256, U256, U256) = contract
            .method("getMetrics", token)?
            .call()
            .await
            .map_err(|e| CrossChainError::ContractError(e.to_string()))?;
        
        // Convert fixed-point numbers to f64
        let apy = apy_raw.as_u64() as f64 / 1e4;
        let utilization_rate = utilization_raw.as_u64() as f64 / 1e4;

        Ok(ProtocolMetrics {
            tvl,
            apy,
            utilization_rate,
            total_borrowed,
            total_supplied,
        })
    }
}

pub struct AaveProtocol {
    provider: Provider<Http>,
    lending_pool: EthersAddress,
}

impl AaveProtocol {
    pub fn new(rpc_url: &str, lending_pool: EthersAddress) -> Result<Self, CrossChainError> {
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| CrossChainError::ProviderError(e.to_string()))?;
        
        Ok(Self {
            provider,
            lending_pool,
        })
    }

    pub async fn borrow(
        &self,
        token: EthersAddress,
        amount: EthersU256,
        interest_rate_mode: u8,
    ) -> Result<Transaction, CrossChainError> {
        let function_selector = [0xc0, 0x4b, 0x8d, 0x59]; // borrow selector
        let mut data = Vec::with_capacity(4 + 32 * 3);
        data.extend_from_slice(&function_selector);
        data.extend_from_slice(&ethers::abi::encode(&[
            token.into_token(),
            amount.into_token(),
            interest_rate_mode.into_token()
        ]));

        let request = TransactionRequest::new()
            .to(self.lending_pool)
            .value(Web3U256::zero())
            .data(data)
            .gas_limit(Web3U256::from(500000));

        crate::send_transaction(&self.provider, request).await
    }
}

#[async_trait(?Send)]
impl DeFiProtocol for AaveProtocol {
    async fn deposit(&self, token: EthersAddress, amount: EthersU256) -> Result<Transaction, CrossChainError> {
        let function_selector = [0xe8, 0xe3, 0x3d, 0x8e]; // deposit selector
        let mut data = Vec::with_capacity(4 + 32 * 2);
        data.extend_from_slice(&function_selector);
        data.extend_from_slice(&ethers::abi::encode(&[
            token.into_token(),
            amount.into_token()
        ]));

        let request = TransactionRequest::new()
            .to(self.lending_pool)
            .value(Web3U256::from(amount.as_u128()))
            .gas_limit(Web3U256::from(300000));

        crate::send_transaction(&self.provider, request).await
    }

    async fn withdraw(&self, token: Address, amount: U256) -> Result<Transaction, CrossChainError> {
        let function_selector = [0x69, 0x32, 0x8d, 0xec]; // withdraw selector
        let mut data = Vec::with_capacity(4 + 32 * 2);
        data.extend_from_slice(&function_selector);
        data.extend_from_slice(&ethers::abi::encode(&[
            token.into_token(),
            amount.into_token()
        ]));

        let request = TransactionRequest::new()
            .to(self.lending_pool)
            .value(web3::types::U256::zero())
            .gas_limit(web3::types::U256::from(300000));

        crate::send_transaction(&self.provider, request).await
    }

    async fn get_metrics(&self, token: Address) -> Result<ProtocolMetrics, CrossChainError> {
        let contract = Contract::new(
            self.lending_pool,
            include_bytes!("../abis/aave_v2.json"),
            Arc::new(self.provider.clone())
        );

        let (tvl, apy, utilization_rate, total_borrowed, total_supplied) = contract
            .method("getReserveData", token)?
            .call()
            .await
            .map_err(|e| CrossChainError::ContractError(e.to_string()))?;

        Ok(ProtocolMetrics {
            tvl,
            apy,
            utilization_rate,
            total_borrowed,
            total_supplied,
        })
    }
}

pub struct CurveProtocol {
    pool: Address,
    registry: Address,
    client: Provider<Http>,
}

impl CurveProtocol {
    pub fn new(pool: Address, registry: Address, rpc_url: &str) -> Result<Self, CrossChainError> {
        let client = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| CrossChainError::ProviderError(e.to_string()))?;
        Ok(Self { pool, registry, client })
    }

    pub async fn exchange(
        &self,
        i: i128,
        j: i128,
        dx: U256,
        min_dy: U256,
    ) -> Result<Transaction, CrossChainError> {
        let function_selector = [0x3d, 0xf0, 0x2a, 0x24]; // exchange selector
        let mut data = Vec::with_capacity(4 + 32 * 4);
        data.extend_from_slice(&function_selector);
        data.extend_from_slice(&ethers::abi::encode(&[
            i.into_token(),
            j.into_token(),
            dx.into_token(),
            min_dy.into_token()
        ]));

        let request = TransactionRequest::new()
            .to(self.pool)
            .data(data)
            .gas_limit(web3::types::U256::from(600000));

        crate::send_transaction(&self.client, request).await
    }
}

#[async_trait(?Send)]
impl DeFiProtocol for CurveProtocol {
    async fn deposit(&self, token: Address, amount: U256) -> Result<Transaction, CrossChainError> {
        let function_selector = [0x6e, 0x55, 0x3f, 0x65]; // add_liquidity selector
        let mut data = Vec::with_capacity(4 + 32 * 2);
        data.extend_from_slice(&function_selector);
        data.extend_from_slice(&ethers::abi::encode(&[
            token.into_token(),
            amount.into_token()
        ]));

        let request = TransactionRequest::new()
            .to(self.pool)
            .data(data)
            .gas_limit(web3::types::U256::from(400000));

        crate::send_transaction(&self.client, request).await
    }

    async fn withdraw(&self, token: Address, amount: U256) -> Result<Transaction, CrossChainError> {
        let function_selector = [0x1a, 0x4b, 0xc1, 0x65]; // remove_liquidity_one_coin selector
        let mut data = Vec::with_capacity(4 + 32 * 2);
        data.extend_from_slice(&function_selector);
        data.extend_from_slice(&ethers::abi::encode(&[
            token.into_token(),
            amount.into_token()
        ]));

        let request = TransactionRequest::new()
            .to(self.pool)
            .data(data)
            .gas_limit(web3::types::U256::from(400000));

        crate::send_transaction(&self.client, request).await
    }

    async fn get_metrics(&self, token: Address) -> Result<ProtocolMetrics, CrossChainError> {
        let contract = ethers::contract::Contract::new(
            self.registry,
            include_bytes!("../abis/curve_registry.json"),
            Arc::new(self.client.clone())
        );

        let (tvl, apy_raw, utilization_raw, total_borrowed, total_supplied): (U256, U256, U256, U256, U256) = contract
            .method("get_pool_stats", (self.pool, token))?
            .call()
            .await
            .map_err(|e| CrossChainError::ContractError(e.to_string()))?;
        
        // Convert fixed-point numbers to f64
        let apy = apy_raw.as_u64() as f64 / 1e4;
        let utilization_rate = utilization_raw.as_u64() as f64 / 1e4;

        Ok(ProtocolMetrics {
            tvl,
            apy,
            utilization_rate,
            total_borrowed,
            total_supplied,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_protocol_creation() {
        let rpc_url = "http://localhost:8545";
        let address = Address::from_str("0x1234567890123456789012345678901234567890").unwrap();
        
        let uniswap = UniswapProtocol::new(rpc_url, address);
        assert!(uniswap.is_ok());
        
        let aave = AaveProtocol::new(rpc_url, address);
        assert!(aave.is_ok());
    }
}