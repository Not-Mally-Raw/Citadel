use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

// Placeholder for oracle implementation
#[derive(Debug, Serialize, Deserialize)]
pub struct PriceData {
    pub token_address: String,
    pub price_usd: Decimal,
    pub timestamp: u64,
    pub source: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::*;

    #[test]
    fn it_works() {
        let price_data = PriceData {
            token_address: "token.near".to_string(),
            price_usd: Decimal::new(1000, 2), // $10.00
            timestamp: 1677649200,
            source: "chainlink".to_string(),
        };
        assert_eq!(price_data.price_usd, Decimal::new(1000, 2));
    }
} 