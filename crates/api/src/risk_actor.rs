use credit_risk::{Belief, CreditRiskFilter, PaymentFeatures};
use tokio::sync::{mpsc, oneshot};

struct AssessRequest {
    prior: Belief,
    features: PaymentFeatures,
    respond_to: oneshot::Sender<credit_risk::Result<Belief>>,
}

/// `Send + Sync + Clone` handle to a `CreditRiskFilter` that lives on its
/// own dedicated OS thread.
///
/// `CreditRiskFilter` itself is `Send` but not `Sync` (see its doc comment
/// for why -- a Burn 0.13 limitation, not something fixable from this
/// crate). Axum's shared `State` requires `Sync`, so instead of putting
/// the filter in `AppState` directly, exactly one thread owns it and every
/// request goes through this channel-backed handle, which *is*
/// `Send + Sync + Clone` because it only ever moves a `Belief` (plain
/// data, no tensors) across the boundary.
#[derive(Clone)]
pub struct RiskActorHandle {
    sender: mpsc::UnboundedSender<AssessRequest>,
}

#[derive(Debug, thiserror::Error)]
pub enum RiskActorError {
    #[error("risk assessment worker thread is not running")]
    Unavailable,
    #[error(transparent)]
    Filter(#[from] credit_risk::CreditRiskError),
}

impl RiskActorHandle {
    /// Spawns the worker thread, builds the `CreditRiskFilter` on it, and
    /// returns once the filter is ready (or propagates its construction
    /// error). Call once at startup and clone the handle into `AppState`.
    pub fn spawn() -> Result<Self, credit_risk::CreditRiskError> {
        let (sender, mut receiver) = mpsc::unbounded_channel::<AssessRequest>();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), credit_risk::CreditRiskError>>();

        std::thread::Builder::new()
            .name("credit-risk-filter".into())
            .spawn(move || {
                let filter = match CreditRiskFilter::new() {
                    Ok(f) => {
                        let _ = ready_tx.send(Ok(()));
                        f
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                while let Some(request) = receiver.blocking_recv() {
                    let result = filter.assess(&request.prior, &request.features);
                    let _ = request.respond_to.send(result);
                }
            })
            .expect("failed to spawn credit-risk-filter thread");

        ready_rx
            .recv()
            .expect("credit-risk-filter thread died before signalling readiness")?;

        Ok(Self { sender })
    }

    /// Runs one predict + update cycle on the worker thread and returns
    /// the posterior belief.
    pub async fn assess(&self, prior: Belief, features: PaymentFeatures) -> Result<Belief, RiskActorError> {
        let (respond_to, response) = oneshot::channel();
        self.sender
            .send(AssessRequest {
                prior,
                features,
                respond_to,
            })
            .map_err(|_| RiskActorError::Unavailable)?;
        response.await.map_err(|_| RiskActorError::Unavailable)?.map_err(RiskActorError::from)
    }
}
