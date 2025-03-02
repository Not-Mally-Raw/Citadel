use thiserror::Error;

#[derive(Error, Debug)]
pub enum MevProtectionError {
    #[error("Provider initialization failed: {0}")]
    ProviderError(String),

    #[error("Transaction analysis failed: {0}")]
    TransactionError(String),

    #[error("Mempool analysis failed: {0}")]
    MempoolError(String),

    #[error("Gas price prediction failed: {0}")]
    GasPredictionError(String),

    #[error("ZK proof generation failed: {0}")]
    ZkProofError(String),

    #[error("Commit-reveal scheme failed: {0}")]
    CommitRevealError(String),

    #[error("Flashbots submission failed: {0}")]
    FlashbotsError(String),
}

pub type Result<T> = std::result::Result<T, MevProtectionError>;
