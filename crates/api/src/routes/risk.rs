use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::dto::RiskAssessmentResponse;
use crate::error::ApiResult;
use crate::risk_service;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/loans/:id/risk/assess", post(assess))
        .route("/loans/:id/risk", get(latest))
        .route("/loans/:id/risk/history", get(history))
}

async fn assess(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Json<RiskAssessmentResponse>> {
    let assessment = risk_service::assess_loan(&state, id).await?;
    Ok(Json(assessment.into()))
}

async fn latest(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Json<Option<RiskAssessmentResponse>>> {
    let assessment = RiskAssessment::find()
        .filter(entity::risk_assessment::Column::LoanId.eq(id))
        .order_by_desc(entity::risk_assessment::Column::AssessedAt)
        .one(&state.db)
        .await?;
    Ok(Json(assessment.map(RiskAssessmentResponse::from)))
}

async fn history(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Json<Vec<RiskAssessmentResponse>>> {
    let assessments = RiskAssessment::find()
        .filter(entity::risk_assessment::Column::LoanId.eq(id))
        .order_by_desc(entity::risk_assessment::Column::AssessedAt)
        .all(&state.db)
        .await?;
    Ok(Json(assessments.into_iter().map(RiskAssessmentResponse::from).collect()))
}
