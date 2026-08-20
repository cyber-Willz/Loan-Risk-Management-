use burn::backend::NdArray;
use burn::tensor::Tensor;
use neural_hmm::{Belief, EmissionEngineConfig, NeuralEmissionEngine, NeuralHmm, TransitionMatrix};

use crate::error::Result;
use crate::features::{PaymentFeatures, FEATURE_DIM};
use crate::states::RiskState;

/// CPU backend for the emission MLP. The model here is small (8 -> 64 ->
/// 32 -> 4) and runs per-loan on assessment requests, so the ndarray
/// backend (no GPU/driver dependency) is the right default for a service
/// deployment; swap the type alias for a GPU backend if assessment volume
/// ever needs it.
pub type CreditBackend = NdArray<f32>;

/// Hand-specified prior transition matrix over
/// `[Current, Watch, Delinquent, Default]`. Rows are `P(next | current)`.
/// Tuned to standard servicing intuition rather than fit from a labeled
/// dataset (none is bundled with this system): current loans mostly stay
/// current, `Default` is treated as absorbing-ish (small chance of a
/// workout moving it to `Delinquent`, none of jumping straight back to
/// `Current`), and every state's most likely one-step transition is to
/// stay put -- deterioration and recovery both happen gradually. Replace
/// with a matrix fit from historical roll-rate data before relying on this
/// in production; this is a reasonable starting prior, not a calibrated
/// model.
fn build_transition_matrix() -> TransitionMatrix {
    TransitionMatrix::with_labels(
        vec![
            vec![0.90, 0.08, 0.02, 0.00],
            vec![0.25, 0.55, 0.18, 0.02],
            vec![0.05, 0.20, 0.55, 0.20],
            vec![0.00, 0.02, 0.08, 0.90],
        ],
        vec![
            "current".into(),
            "watch".into(),
            "delinquent".into(),
            "default".into(),
        ],
    )
    .expect("hand-specified transition matrix is row-stochastic by construction")
}

/// Uniform prior over the four states -- the correct starting belief for
/// a loan with no prior assessment on record. Free function (not a
/// `CreditRiskFilter` method): it needs no tensor/network state, so
/// callers that only need a prior belief (e.g. before routing through the
/// actor in `api::risk_actor`, since `CreditRiskFilter` itself is not
/// `Sync` -- see its doc comment) don't need a filter instance to get one.
pub fn initial_belief() -> Result<Belief> {
    Ok(Belief::uniform(RiskState::COUNT)?)
}

/// Most probable state and its posterior probability. Free function for
/// the same reason as [`initial_belief`]: pure `Belief` arithmetic, no
/// tensor/network state involved.
pub fn state_of(belief: &Belief) -> (RiskState, f32) {
    let (idx, p) = belief.argmax();
    (RiskState::from_index(idx), p)
}

/// Wraps `neural_hmm::NeuralHmm` with the loan-risk state space, transition
/// prior, and feature encoding, so callers deal in [`PaymentFeatures`] and
/// [`RiskState`] instead of raw tensors and state indices.
///
/// **Not `Sync`.** Burn 0.13's `Param<Tensor<_>>` holds a
/// `Box<dyn Fn(..) -> Tensor + Send>` in its (statically always-present,
/// even once initialized) lazy-init slot, and that closure type has no
/// `Sync` bound -- so nothing containing a `Param` can ever be `Sync`,
/// regardless of feature flags. It *is* `Send`. In a multi-threaded async
/// server this means a `CreditRiskFilter` cannot sit behind `Arc` in
/// shared state directly; give exactly one thread ownership of it and
/// route requests to that thread (see `api::risk_actor::RiskActorHandle`
/// for the pattern this system uses).
pub struct CreditRiskFilter {
    hmm: NeuralHmm<CreditBackend>,
    device: <CreditBackend as burn::tensor::backend::Backend>::Device,
}

impl CreditRiskFilter {
    /// Builds a filter with a freshly-initialized (untrained) emission
    /// network. The emission weights are random at construction; see the
    /// module docs on `build_transition_matrix` for the same caveat this
    /// implies for `assess` output until the network is trained/loaded
    /// from a checkpoint via `neural_hmm`'s `NeuralEmissionEngine::load`.
    pub fn new() -> Result<Self> {
        let device = Default::default();
        let config = EmissionEngineConfig::new(FEATURE_DIM, RiskState::COUNT);
        let emission_engine = NeuralEmissionEngine::<CreditBackend>::new(&device, &config);
        let hmm = NeuralHmm::new(emission_engine, build_transition_matrix())?;
        Ok(Self { hmm, device })
    }

    /// Runs one predict + update cycle from `prior` given this
    /// assessment's `features`, returning the posterior belief.
    pub fn assess(&self, prior: &Belief, features: &PaymentFeatures) -> Result<Belief> {
        let vector = features.to_vector();
        let tensor = Tensor::<CreditBackend, 2>::from_data([vector], &self.device);
        Ok(self.hmm.filter_step(prior, tensor)?)
    }
}
