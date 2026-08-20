use serde::{Deserialize, Serialize};

/// The four hidden credit-risk states tracked by the filter. Ordering is
/// load-bearing: it must match the row/column order of the transition
/// matrix built in `filter::build_transition_matrix`, and index `n` here
/// is state `n` in every `Belief` this crate produces.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskState {
    /// Payments on schedule, no material stress signals.
    Current = 0,
    /// Early stress signals (a late payment, rising utilization) but not
    /// yet delinquent — the state a servicer should proactively reach out
    /// on.
    Watch = 1,
    /// Materially behind on payments.
    Delinquent = 2,
    /// Effectively unrecoverable without restructuring/collections.
    Default = 3,
}

impl RiskState {
    pub const COUNT: usize = 4;

    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => RiskState::Current,
            1 => RiskState::Watch,
            2 => RiskState::Delinquent,
            _ => RiskState::Default,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RiskState::Current => "current",
            RiskState::Watch => "watch",
            RiskState::Delinquent => "delinquent",
            RiskState::Default => "default",
        }
    }
}
