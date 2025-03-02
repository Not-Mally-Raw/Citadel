use crate::{
    CrossChainError,
    protocols::{ProtocolMetrics, DeFiProtocol},
};
use aurora_sdk::aurora_engine_types::{types::Address, U256};
use ethers::types::Transaction;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MockProvider {
    balances: Arc<RwLock<HashMap<Address, U256>>>,
    metrics: Arc<RwLock<HashMap<Address, ProtocolMetrics>>>,
    transactions: Arc<RwLock<Vec<Transaction>>>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            balances: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn set_balance(&self, token: Address, amount: U256) {
        self.balances.write().await.insert(token, amount);
    }

    pub async fn set_metrics(&self, token: Address, metrics: ProtocolMetrics) {
        self.metrics.write().await.insert(token, metrics);
    }

    pub async fn get_transaction_count(&self) -> usize {
        self.transactions.read().await.len()
    }

    pub async fn get_last_transaction(&self) -> Option<Transaction> {
        self.transactions.read().await.last().cloned()
    }
}

#[async_trait]
impl DeFiProtocol for MockProvider {
    async fn deposit(&self, token: Address, amount: U256) -> Result<Transaction, CrossChainError> {
        let mut balances = self.balances.write().await;
        let current = balances.get(&token).copied().unwrap_or_default();
        balances.insert(token, current + amount);

        let tx = Transaction {
            from: Address::zero(),
            to: Some(token),
            value: amount,
            ..Default::default()
        };

        self.transactions.write().await.push(tx.clone());
        Ok(tx)
    }

    async fn withdraw(&self, token: Address, amount: U256) -> Result<Transaction, CrossChainError> {
        let mut balances = self.balances.write().await;
        let current = balances.get(&token).copied().unwrap_or_default();
        
        if current < amount {
            return Err(CrossChainError::ProtocolError("Insufficient balance".to_string()));
        }

        balances.insert(token, current - amount);

        let tx = Transaction {
            from: token,
            to: Some(Address::zero()),
            value: amount,
            ..Default::default()
        };

        self.transactions.write().await.push(tx.clone());
        Ok(tx)
    }

    async fn get_metrics(&self, token: Address) -> Result<ProtocolMetrics, CrossChainError> {
        self.metrics
            .read()
            .await
            .get(&token)
            .cloned()
            .ok_or_else(|| CrossChainError::ProtocolError("No metrics available".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::block_on;

    #[test]
    fn test_mock_provider() {
        block_on(async {
            let provider = MockProvider::new();
            let token = Address::from_low_u64_be(1);
            let amount = U256::from(1_000_000);

            // Test deposit
            provider.deposit(token, amount).await.unwrap();
            assert_eq!(provider.get_transaction_count().await, 1);

            // Test balance
            provider.set_balance(token, amount).await;
            let withdraw_result = provider.withdraw(token, amount).await;
            assert!(withdraw_result.is_ok());

            // Test metrics
            let metrics = ProtocolMetrics {
                tvl: amount,
                apy: 0.05,
                utilization_rate: 0.8,
                total_borrowed: amount / 2,
                total_supplied: amount,
            };
            provider.set_metrics(token, metrics.clone()).await;

            let fetched_metrics = provider.get_metrics(token).await.unwrap();
            assert_eq!(fetched_metrics.tvl, metrics.tvl);
            assert_eq!(fetched_metrics.apy, metrics.apy);
        });
    }
} 