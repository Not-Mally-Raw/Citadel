use thiserror::Error;
use near_sdk::AccountId;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance {
        required: u128,
        available: u128,
    },

    #[error("Amount below minimum: {0}")]
    BelowMinimum(u128),

    #[error("Amount above maximum: {0}")]
    AboveMaximum(u128),

    #[error("Invalid account: {0}")]
    InvalidAccount(AccountId),

    #[error("Operation not permitted: {0}")]
    NotPermitted(String),

    #[error("Oracle error: {0}")]
    OracleError(String),

    #[error("Bridge error: {0}")]
    BridgeError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Security check failed: {0}")]
    SecurityCheckFailed(String),

    #[error("Slippage exceeded: expected {expected}, actual {actual}")]
    SlippageExceeded {
        expected: u128,
        actual: u128,
    },

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Contract call failed: {0}")]
    ContractCallFailed(String),
}

pub type VaultResult<T> = Result<T, VaultError>;

// Helper functions for common error scenarios
pub mod helpers {
    use super::*;

    pub fn check_amount(
        amount: u128,
        min: u128,
        max: u128
    ) -> VaultResult<()> {
        if amount < min {
            return Err(VaultError::BelowMinimum(min));
        }
        if amount > max {
            return Err(VaultError::AboveMaximum(max));
        }
        Ok(())
    }

    pub fn check_slippage(
        expected: u128,
        actual: u128,
        max_slippage_bps: u32
    ) -> VaultResult<()> {
        let slippage = if actual > expected {
            actual - expected
        } else {
            expected - actual
        };

        if slippage * 10000 > expected * max_slippage_bps as u128 {
            return Err(VaultError::SlippageExceeded {
                expected,
                actual,
            });
        }
        Ok(())
    }

    pub fn check_balance(
        required: u128,
        available: u128
    ) -> VaultResult<()> {
        if required > available {
            return Err(VaultError::InsufficientBalance {
                required,
                available,
            });
        }
        Ok(())
    }
}

// Macros for error handling
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr) => {
        if !($cond) {
            return Err($err);
        }
    };
}

#[macro_export]
macro_rules! require_msg {
    ($cond:expr, $err:expr) => {
        if !($cond) {
            return Err(VaultError::NotPermitted($err.to_string()));
        }
    };
}

// Example usage:
/*
fn transfer_tokens(from: AccountId, to: AccountId, amount: u128) -> VaultResult<()> {
    ensure!(amount > 0, VaultError::BelowMinimum(1));
    require_msg!(
        env::predecessor_account_id() == from,
        "Only account owner can transfer"
    );
    
    helpers::check_balance(amount, get_balance(&from))?;
    // ... perform transfer
    Ok(())
}
*/ 