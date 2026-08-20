use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use entity::prelude::*;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::Deserialize;
use uuid::Uuid;

use crate::dto::{CreateBorrowerRequest, RelatedBorrowersResponse};
use crate::error::{ApiError, ApiResult};
use crate::network_service;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/borrowers", post(create_borrower).get(list_borrowers))
        .route("/borrowers/:id", get(get_borrower))
        .route("/borrowers/:id/relationships", get(list_relationships))
        .route("/borrowers/:id/related", get(related_borrowers))
}

async fn create_borrower(
    State(state): State<AppState>,
    Json(req): Json<CreateBorrowerRequest>,
) -> ApiResult<Json<BorrowerModel>> {
    let now = Utc::now();
    let model = entity::borrower::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(req.name),
        borrower_type: Set(req.borrower_type),
        national_id: Set(req.national_id),
        employer: Set(req.employer),
        address: Set(req.address),
        email: Set(req.email),
        phone: Set(req.phone),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
    };
    let saved = model.insert(&state.db).await?;
    Ok(Json(saved))
}

async fn list_borrowers(State(state): State<AppState>) -> ApiResult<Json<Vec<BorrowerModel>>> {
    let borrowers = Borrower::find().all(&state.db).await?;
    Ok(Json(borrowers))
}

async fn get_borrower(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Json<BorrowerModel>> {
    let borrower = Borrower::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("borrower {id}")))?;
    Ok(Json(borrower))
}

async fn list_relationships(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<RelationshipLinkModel>>> {
    use sea_orm::{ColumnTrait, QueryFilter};
    let links = RelationshipLink::find()
        .filter(
            entity::relationship_link::Column::SourceBorrowerId
                .eq(id)
                .or(entity::relationship_link::Column::TargetBorrowerId.eq(id)),
        )
        .all(&state.db)
        .await?;
    Ok(Json(links))
}

#[derive(Debug, Deserialize)]
struct RelatedQuery {
    #[serde(default = "default_depth")]
    depth: usize,
}

fn default_depth() -> usize {
    2
}

async fn related_borrowers(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<RelatedQuery>,
) -> ApiResult<Json<RelatedBorrowersResponse>> {
    let related = network_service::related_borrowers(&state, id, q.depth).await?;
    Ok(Json(RelatedBorrowersResponse {
        borrower_id: id,
        related,
    }))
}
