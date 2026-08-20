pub mod borrowers;
pub mod loans;
pub mod network;
pub mod payments;
pub mod relationships;
pub mod risk;

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(borrowers::router())
        .merge(loans::router())
        .merge(payments::router())
        .merge(relationships::router())
        .merge(risk::router())
        .merge(network::router())
}
