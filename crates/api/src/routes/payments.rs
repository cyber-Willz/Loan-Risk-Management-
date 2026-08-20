use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use entity::prelude::*;
use sea_orm::prelude::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use uuid::Uuid;

use crate::dto::RecordPaymentRequest;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/loans/:loan_id/payments",
        post(record_payment).get(list_payments),
    )
}

/// Derives status/days_late from due/paid dates and amounts: on time (or
/// early) if paid on/before the due date and in full, late if paid after
/// the due date, partial if paid in full but for less than amount_due, and
/// still-scheduled/missed based on whether today has passed the due date
/// with nothing paid yet.
fn derive_status_and_lateness(
    due_date: chrono::NaiveDate,
    paid_date: Option<chrono::NaiveDate>,
    amount_due: Decimal,
    amount_paid: Decimal,
) -> (PaymentStatus, Option<i32>) {
    match paid_date {
        Some(paid) => {
            let days_late = (paid - due_date).num_days().max(0) as i32;
            if amount_paid < amount_due {
                (PaymentStatus::Partial, Some(days_late))
            } else if paid > due_date {
                (PaymentStatus::PaidLate, Some(days_late))
            } else {
                (PaymentStatus::PaidOnTime, Some(0))
            }
        }
        None => {
            let today = Utc::now().date_naive();
            if today > due_date {
                (PaymentStatus::Missed, Some((today - due_date).num_days() as i32))
            } else {
                (PaymentStatus::Scheduled, None)
            }
        }
    }
}

async fn record_payment(
    State(state): State<AppState>,
    Path(loan_id): Path<Uuid>,
    Json(req): Json<RecordPaymentRequest>,
) -> ApiResult<Json<PaymentModel>> {
    if Loan::find_by_id(loan_id).one(&state.db).await?.is_none() {
        return Err(ApiError::NotFound(format!("loan {loan_id}")));
    }

    let (status, days_late) =
        derive_status_and_lateness(req.due_date, req.paid_date, req.amount_due, req.amount_paid);

    let model = entity::payment::ActiveModel {
        id: Set(Uuid::new_v4()),
        loan_id: Set(loan_id),
        due_date: Set(req.due_date),
        paid_date: Set(req.paid_date),
        amount_due: Set(req.amount_due),
        amount_paid: Set(req.amount_paid),
        status: Set(status),
        days_late: Set(days_late),
        created_at: Set(Utc::now().into()),
    };
    let saved = model.insert(&state.db).await?;
    Ok(Json(saved))
}

async fn list_payments(State(state): State<AppState>, Path(loan_id): Path<Uuid>) -> ApiResult<Json<Vec<PaymentModel>>> {
    let payments = Payment::find()
        .filter(entity::payment::Column::LoanId.eq(loan_id))
        .order_by_desc(entity::payment::Column::DueDate)
        .all(&state.db)
        .await?;
    Ok(Json(payments))
}
