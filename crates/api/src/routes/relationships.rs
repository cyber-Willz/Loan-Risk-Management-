use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use entity::prelude::*;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use uuid::Uuid;

use crate::dto::CreateRelationshipRequest;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/relationships", post(create_relationship))
}

async fn create_relationship(
    State(state): State<AppState>,
    Json(req): Json<CreateRelationshipRequest>,
) -> ApiResult<Json<RelationshipLinkModel>> {
    if req.source_borrower_id == req.target_borrower_id {
        return Err(ApiError::BadRequest("a borrower cannot be related to themselves".into()));
    }
    for id in [req.source_borrower_id, req.target_borrower_id] {
        if Borrower::find_by_id(id).one(&state.db).await?.is_none() {
            return Err(ApiError::BadRequest(format!("borrower {id} does not exist")));
        }
    }
    if let Some(loan_id) = req.loan_id {
        if Loan::find_by_id(loan_id).one(&state.db).await?.is_none() {
            return Err(ApiError::BadRequest(format!("loan {loan_id} does not exist")));
        }
    }
    if !req.weight.is_finite() || req.weight < 0.0 {
        return Err(ApiError::BadRequest("weight must be a non-negative finite number".into()));
    }

    let model = entity::relationship_link::ActiveModel {
        id: Set(Uuid::new_v4()),
        source_borrower_id: Set(req.source_borrower_id),
        target_borrower_id: Set(req.target_borrower_id),
        relation_type: Set(req.relation_type),
        loan_id: Set(req.loan_id),
        weight: Set(req.weight),
        created_at: Set(Utc::now().into()),
    };
    let saved = model.insert(&state.db).await?;
    Ok(Json(saved))
}
