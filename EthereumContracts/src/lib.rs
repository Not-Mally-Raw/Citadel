use ethers::{
    types::{Address as EthersAddress, U256 as EthersU256, TransactionRequest as EthersTransactionRequest, H256 as EthersH256},
    providers::{Provider, Http},
    middleware::Middleware,
};
use web3::types::{Address as Web3Address, U256 as Web3U256, Bytes, Transaction, H256 as Web3H256, U64 as Web3U64};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

// Type conversions between ethers and web3
pub trait IntoWeb3<T> {
    fn into_web3(self) -> T;
}

pub trait IntoEthers<T> {
    fn into_ethers(self) -> T;
}

impl IntoWeb3<Web3Address> for EthersAddress {
    fn into_web3(self) -> Web3Address {
        Web3Address::from_slice(self.as_bytes())
    }
}

impl IntoEthers<EthersAddress> for Web3Address {
    fn into_ethers(self) -> EthersAddress {
        EthersAddress::from_slice(self.as_bytes())
    }
}

impl IntoWeb3<Web3U256> for EthersU256 {
    fn into_web3(self) -> Web3U256 {
        Web3U256::from_dec_str(&self.to_string()).unwrap_or_default()
    }
}

impl IntoEthers<EthersU256> for Web3U256 {
    fn into_ethers(self) -> EthersU256 {
        EthersU256::from_dec_str(&self.to_string()).unwrap_or_default()
    }
}

impl IntoWeb3<Web3H256> for ethers::types::H256 {
    fn into_web3(self) -> Web3H256 {
        Web3H256::from_slice(&self.0)
    }
}

impl IntoEthers<EthersH256> for Web3H256 {
    fn into_ethers(self) -> EthersH256 {
        EthersH256::from_slice(&self.0)
    }
}

pub mod bridge;
pub mod protocols;
pub mod utils;

#[derive(Error, Debug)]
pub enum CrossChainError {
    #[error("Invalid amount")]
    InvalidAmount,
    #[error("Invalid address")]
    InvalidAddress,
    #[error("Transaction failed")]
    TransactionFailed(String),
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Contract error: {0}")]
    ContractError(String),
    #[error("ABI error: {0}")]
    AbiError(String),
}

impl From<ethers::abi::Error> for CrossChainError {
    fn from(err: ethers::abi::Error) -> Self {
        CrossChainError::AbiError(err.to_string())
    }
}

impl From<ethers::contract::AbiError> for CrossChainError {
    fn from(err: ethers::contract::AbiError) -> Self {
        CrossChainError::AbiError(err.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ProtocolType {
    Uniswap,
    Aave,
    Compound,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionRequest {
    pub to: Option<EthersAddress>,
    pub data: Vec<u8>,
    pub value: Web3U256,
    pub gas_limit: Web3U256,
}

impl TransactionRequest {
    pub fn new() -> Self {
        Self {
            to: None,
            data: Vec::new(),
            value: Web3U256::zero(),
            gas_limit: Web3U256::from(21000),
        }
    }

    pub fn to(mut self, to: EthersAddress) -> Self {
        self.to = Some(to);
        self
    }

    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    pub fn value(mut self, value: Web3U256) -> Self {
        self.value = value;
        self
    }

    pub fn gas_limit(mut self, gas_limit: Web3U256) -> Self {
        self.gas_limit = gas_limit;
        self
    }
}

pub async fn send_transaction(
    provider: &Provider<Http>,
    request: TransactionRequest,
) -> Result<Transaction, CrossChainError> {
    // Convert web3 types to ethers types for the transaction
    let tx = EthersTransactionRequest::new()
        .to(request.to.unwrap_or_default())
        .data(request.data.clone())
        .value(request.value.into_ethers())
        .gas(request.gas_limit.into_ethers());

    let pending_tx = provider
        .send_transaction(tx, None)
        .await
        .map_err(|e| CrossChainError::TransactionFailed(e.to_string()))?;

    let receipt = pending_tx
        .await
        .map_err(|e| CrossChainError::TransactionFailed(e.to_string()))?;

    let receipt = receipt.ok_or_else(|| CrossChainError::TransactionFailed("No receipt".to_string()))?;

    // Convert ethers types to web3 types for the response
    let tx_hash: Web3H256 = receipt.transaction_hash.into_web3();
    let block_hash = receipt.block_hash.map(|h| h.into_web3());
    let nonce = Web3U256::zero(); // Transaction receipts don't have nonce
    
    // Safe conversions for numeric types
    let block_number = receipt.block_number.map(|n| {
        let num = n.as_u64();
        Web3U64::from(num)
    });
    
    let tx_index = receipt.transaction_index.map(|i| {
        let num = i.as_u64();
        Web3U64::from(num)
    });
    
    let from = Some(receipt.from.into_web3());
    let to = receipt.to.map(|addr| addr.into_web3());
    let value = Web3U256::zero();
    
    let gas_price = receipt.effective_gas_price.map(|p| {
        let num = p.as_u128();
        Web3U256::from(num)
    });
    
    let gas = Web3U256::from(receipt.gas_used.unwrap_or_default().as_u128());
    
    let tx_type = receipt.transaction_type.map(|t| {
        let num = t.as_u64();
        Web3U64::from(num)
    });

    Ok(Transaction {
        hash: tx_hash,
        nonce,
        block_hash,
        block_number,
        transaction_index: tx_index,
        from,
        to,
        value,
        gas_price,
        gas,
        input: Bytes(request.data),
        v: None,
        r: None,
        s: None,
        raw: None,
        transaction_type: tx_type,
        access_list: None,
        max_priority_fee_per_gas: None,
        max_fee_per_gas: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_request() {
        let request = TransactionRequest::new()
            .to(Address::zero())
            .value(U256::from(1000))
            .gas_limit(U256::from(50000));

        assert_eq!(request.to.unwrap(), Address::zero());
        assert_eq!(request.value, U256::from(1000));
        assert_eq!(request.gas_limit, U256::from(50000));
    }
} 