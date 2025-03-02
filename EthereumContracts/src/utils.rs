use ethers::types::{Address, U256};
use web3::types::Transaction;
use crate::CrossChainError;

pub fn validate_amount(amount: U256) -> Result<(), CrossChainError> {
    if amount == U256::zero() {
        return Err(CrossChainError::InvalidAmount);
    }
    Ok(())
}

pub fn validate_address(address: Address) -> Result<(), CrossChainError> {
    if address == Address::zero() {
        return Err(CrossChainError::InvalidAddress);
    }
    Ok(())
}

pub fn format_transaction(tx: Transaction) -> String {
    format!("Transaction: hash={:?}, from={:?}, to={:?}, value={:?}",
        tx.hash, tx.from, tx.to, tx.value)
} 