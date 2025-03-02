use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrossChainError {
    #[error("Invalid amount provided")]
    InvalidAmount,

    #[error("Invalid address provided")]
    InvalidAddress,

    #[error("Transaction failed: {0}")]
    TransactionError(String),

    #[error("Chain connection error: {0}")]
    ConnectionError(String),

    #[error("Contract interaction error: {0}")]
    ContractError(String),
}