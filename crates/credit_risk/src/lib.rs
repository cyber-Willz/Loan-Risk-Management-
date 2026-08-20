//! Loan-risk-specific wrapper around the `neural_hmm` crate: defines the
//! four-state credit risk state space ([`states::RiskState`]), the
//! observation feature encoding ([`features::PaymentFeatures`]), and a
//! domain transition prior, so the rest of the system works in loan-risk
//! terms rather than raw HMM primitives.

pub mod error;
pub mod features;
pub mod filter;
pub mod states;

pub use error::{CreditRiskError, Result};
pub use features::{PaymentFeatures, FEATURE_DIM};
pub use filter::{initial_belief, state_of, CreditRiskFilter};
pub use neural_hmm::Belief;
pub use states::RiskState;
