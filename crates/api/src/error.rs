use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
    #[error("credit risk engine error: {0}")]
    Risk(#[from] credit_risk::CreditRiskError),
    #[error("risk assessment worker error: {0}")]
    RiskActor(#[from] crate::risk_actor::RiskActorError),
    #[error("credit network error: {0}")]
    Network(#[from] credit_network::CreditNetworkError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Network(credit_network::CreditNetworkError::TooFewBorrowers(_))
            | ApiError::Network(credit_network::CreditNetworkError::InvalidClusterCount { .. }) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            ApiError::Db(_) | ApiError::Risk(_) | ApiError::Network(_) | ApiError::RiskActor(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        let body = Json(json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
