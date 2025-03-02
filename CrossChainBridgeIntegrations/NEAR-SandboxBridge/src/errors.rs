#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("Invalid transfer amount: {0}")]
    InvalidAmount(String),

    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),

    #[error("Cross-chain verification failed: {0}")]
    CrossChainVerification(String),

    #[error("Contract call failed: {0}")]
    ContractCall(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type BridgeResult<T> = Result<T, BridgeError>;