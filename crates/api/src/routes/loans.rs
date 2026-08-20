use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Months, Utc};
use entity::prelude::*;
use sea_orm::{ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::dto::{CreateLoanRequest, ListLoansQuery};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/loans", post(create_loan).get(list_loans))
        .route("/loans/:id", get(get_loan))
}

async fn create_loan(State(state): State<AppState>, Json(req): Json<CreateLoanRequest>) -> ApiResult<Json<LoanModel>> {
    if Borrower::find_by_id(req.borrower_id).one(&state.db).await?.is_none() {
        return Err(ApiError::BadRequest(format!("borrower {} does not exist", req.borrower_id)));
    }
    if req.term_months <= 0 {
        return Err(ApiError::BadRequest("term_months must be positive".into()));
    }

    let maturity_date = req
        .origination_date
        .checked_add_months(Months::new(req.term_months as u32))
        .ok_or_else(|| ApiError::BadRequest("origination_date + term_months overflowed".into()))?;

    let now = Utc::now();
    let model = entity::loan::ActiveModel {
        id: Set(Uuid::new_v4()),
        borrower_id: Set(req.borrower_id),
        principal_amount: Set(req.principal_amount),
        interest_rate_bps: Set(req.interest_rate_bps),
        term_months: Set(req.term_months),
        status: Set(LoanStatus::Active),
        origination_date: Set(req.origination_date),
        maturity_date: Set(maturity_date),
        collateral_value: Set(req.collateral_value),
        purpose: Set(req.purpose),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
    };
    let saved = model.insert(&state.db).await?;
    Ok(Json(saved))
}

async fn list_loans(State(state): State<AppState>, Query(q): Query<ListLoansQuery>) -> ApiResult<Json<Vec<LoanModel>>> {
    let mut condition = Condition::all();
    if let Some(borrower_id) = q.borrower_id {
        condition = condition.add(entity::loan::Column::BorrowerId.eq(borrower_id));
    }
    if let Some(status) = q.status {
        condition = condition.add(entity::loan::Column::Status.eq(status));
    }
    let loans = Loan::find().filter(condition).all(&state.db).await?;
    Ok(Json(loans))
}

async fn get_loan(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Json<LoanModel>> {
    let loan = Loan::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("loan {id}")))?;
    Ok(Json(loan))
}
