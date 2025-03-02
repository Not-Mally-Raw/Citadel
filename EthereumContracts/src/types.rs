use ethers::types::{TransactionReceipt, U256};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Default, Clone)]
pub struct MempoolStats {
    pub total_transactions: u64,
    pub avg_gas_price: U256,
    pub max_gas_price: U256,
    pub min_gas_price: U256,
    pub pending_value: U256,
    pub timestamp: u64,
}

impl MempoolStats {
    pub fn new() -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            ..Default::default()
        }
    }

    pub fn update(&mut self, receipt: &TransactionReceipt) {
        self.total_transactions += 1;
        
        if let Some(gas_price) = receipt.effective_gas_price {
            self.avg_gas_price = (self.avg_gas_price * U256::from(self.total_transactions - 1) + gas_price) 
                / U256::from(self.total_transactions);
                
            self.max_gas_price = std::cmp::max(self.max_gas_price, gas_price);
            
            if self.min_gas_price == U256::zero() {
                self.min_gas_price = gas_price;
            } else {
                self.min_gas_price = std::cmp::min(self.min_gas_price, gas_price);
            }
        }
        
        if let Some(value) = receipt.transaction_fee {
            self.pending_value += value;
        }
    }

    pub fn combine(self, other: Self) -> Self {
        let total = self.total_transactions + other.total_transactions;
        if total == 0 {
            return Self::new();
        }

        let weighted_avg = (self.avg_gas_price * U256::from(self.total_transactions) 
            + other.avg_gas_price * U256::from(other.total_transactions)) / U256::from(total);

        Self {
            total_transactions: total,
            avg_gas_price: weighted_avg,
            max_gas_price: std::cmp::max(self.max_gas_price, other.max_gas_price),
            min_gas_price: if self.min_gas_price == U256::zero() {
                other.min_gas_price
            } else if other.min_gas_price == U256::zero() {
                self.min_gas_price
            } else {
                std::cmp::min(self.min_gas_price, other.min_gas_price)
            },
            pending_value: self.pending_value + other.pending_value,
            timestamp: std::cmp::max(self.timestamp, other.timestamp),
        }
    }
}
