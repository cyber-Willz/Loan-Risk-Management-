use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};

use crate::dto::{AnalyzeNetworkRequest, ContagionResultResponse};
use crate::error::ApiResult;
use crate::network_service;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/network/analyze", post(analyze))
}

async fn analyze(
    State(state): State<AppState>,
    Json(req): Json<AnalyzeNetworkRequest>,
) -> ApiResult<Json<Vec<ContagionResultResponse>>> {
    let results = network_service::analyze(&state, req).await?;
    Ok(Json(results.into_iter().map(ContagionResultResponse::from).collect()))
}
