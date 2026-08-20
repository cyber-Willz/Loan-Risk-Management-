use thiserror::Error;

#[derive(Debug, Error)]
pub enum CreditNetworkError {
    #[error("underlying hypergraph error: {0}")]
    Hypergraph(#[from] spectral_hypergraph::error::HypergraphError),
    #[error("ontology engine error: {0}")]
    Ontology(String),
    #[error("network has fewer than 2 borrowers ({0}); spectral analysis needs at least 2 connected nodes")]
    TooFewBorrowers(usize),
    #[error("requested cluster count {k} exceeds borrower count {n}")]
    InvalidClusterCount { k: usize, n: usize },
}

pub type Result<T> = std::result::Result<T, CreditNetworkError>;
