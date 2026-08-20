use thiserror::Error;

#[derive(Debug, Error)]
pub enum CreditRiskError {
    #[error("neural HMM error: {0}")]
    Hmm(#[from] neural_hmm::HmmError),
}

pub type Result<T> = std::result::Result<T, CreditRiskError>;
