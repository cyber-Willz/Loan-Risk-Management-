use chrono::{Datelike, Utc};
use credit_risk::{Belief, PaymentFeatures};
use entity::prelude::*;
use sea_orm::prelude::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// How many most-recent payments feed the feature window. Matches the
/// "last 6" framing documented on `PaymentFeatures`.
const RECENT_PAYMENT_WINDOW: u64 = 6;

async fn build_features(
    state: &AppState,
    loan: &LoanModel,
    network_contagion_score: f64,
) -> ApiResult<PaymentFeatures> {
    let recent_payments: Vec<PaymentModel> = Payment::find()
        .filter(entity::payment::Column::LoanId.eq(loan.id))
        .order_by_desc(entity::payment::Column::DueDate)
        .limit(RECENT_PAYMENT_WINDOW)
        .all(&state.db)
        .await?;

    let recent_3 = &recent_payments[..recent_payments.len().min(3)];

    let days_late = |p: &PaymentModel| p.days_late.unwrap_or(0).max(0) as f32;

    let avg_days_late_recent = if recent_3.is_empty() {
        0.0
    } else {
        recent_3.iter().map(days_late).sum::<f32>() / recent_3.len() as f32
    };
    let max_days_late_6 = recent_payments.iter().map(days_late).fold(0.0f32, f32::max);
    let missed_count_6 = recent_payments
        .iter()
        .filter(|p| p.status == PaymentStatus::Missed)
        .count() as f32;
    let on_time_count_6 = recent_payments
        .iter()
        .filter(|p| p.status == PaymentStatus::PaidOnTime)
        .count() as f32;

    let payment_ratio_recent = if recent_3.is_empty() {
        1.0
    } else {
        let ratios: Vec<f32> = recent_3
            .iter()
            .filter(|p| p.amount_due > Decimal::ZERO)
            .map(|p| (p.amount_paid / p.amount_due).to_string().parse::<f32>().unwrap_or(1.0))
            .collect();
        if ratios.is_empty() {
            1.0
        } else {
            ratios.iter().sum::<f32>() / ratios.len() as f32
        }
    };

    let collateral_utilization = match loan.collateral_value {
        Some(cv) if cv > Decimal::ZERO => {
            let ratio = (loan.principal_amount / cv).to_string().parse::<f32>().unwrap_or(0.0);
            ratio.clamp(0.0, 3.0)
        }
        _ => 0.0,
    };

    let today = Utc::now().date_naive();
    let months_elapsed = ((today.year() - loan.origination_date.year()) * 12
        + (today.month() as i32 - loan.origination_date.month() as i32))
        .max(0) as f32;
    let loan_age_fraction = if loan.term_months > 0 {
        (months_elapsed / loan.term_months as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };

    Ok(PaymentFeatures {
        avg_days_late_recent,
        max_days_late_6,
        missed_count_6,
        on_time_count_6,
        payment_ratio_recent,
        collateral_utilization,
        loan_age_fraction,
        network_contagion_score: network_contagion_score as f32,
    })
}

async fn latest_network_contagion_score(state: &AppState, borrower_id: Uuid) -> ApiResult<f64> {
    let latest: Option<NetworkSnapshotModel> = NetworkSnapshot::find()
        .filter(entity::network_snapshot::Column::BorrowerId.eq(borrower_id))
        .order_by_desc(entity::network_snapshot::Column::ComputedAt)
        .one(&state.db)
        .await?;
    Ok(latest.map(|s| s.contagion_score).unwrap_or(0.0))
}

async fn prior_belief(state: &AppState, loan_id: Uuid) -> ApiResult<Belief> {
    let latest: Option<RiskAssessmentModel> = RiskAssessment::find()
        .filter(entity::risk_assessment::Column::LoanId.eq(loan_id))
        .order_by_desc(entity::risk_assessment::Column::AssessedAt)
        .one(&state.db)
        .await?;

    match latest {
        Some(assessment) => {
            let values: Vec<f32> = serde_json::from_value(assessment.belief).unwrap_or_default();
            if values.len() == credit_risk::RiskState::COUNT {
                Belief::new(values).or_else(|_| credit_risk::initial_belief())
            } else {
                credit_risk::initial_belief()
            }
            .map_err(ApiError::from)
        }
        None => Ok(credit_risk::initial_belief()?),
    }
}

/// Runs one HMM assessment cycle for `loan_id`: loads the prior belief
/// (or starts uniform), builds observation features from recent payment
/// history plus the borrower's latest network contagion score, filters,
/// and persists the resulting `risk_assessments` row. Also nudges the
/// loan's `status` to match the new state, unless the loan is already in
/// a terminal state (`closed`/`charged_off`).
pub async fn assess_loan(state: &AppState, loan_id: Uuid) -> ApiResult<RiskAssessmentModel> {
    let loan = Loan::find_by_id(loan_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("loan {loan_id}")))?;

    let contagion_score = latest_network_contagion_score(state, loan.borrower_id).await?;
    let features = build_features(state, &loan, contagion_score).await?;
    let prior = prior_belief(state, loan_id).await?;
    let posterior = state.risk_actor.assess(prior, features).await?;
    let (risk_state, probability) = credit_risk::state_of(&posterior);

    let belief_values: Vec<f64> = posterior.as_slice().iter().map(|&v| v as f64).collect();

    let entity_state = match risk_state {
        credit_risk::RiskState::Current => RiskState::Current,
        credit_risk::RiskState::Watch => RiskState::Watch,
        credit_risk::RiskState::Delinquent => RiskState::Delinquent,
        credit_risk::RiskState::Default => RiskState::Default,
    };

    let assessment = entity::risk_assessment::ActiveModel {
        id: Set(Uuid::new_v4()),
        loan_id: Set(loan_id),
        assessed_at: Set(Utc::now().into()),
        state: Set(entity_state),
        state_probability: Set(probability as f64),
        belief: Set(serde_json::to_value(&belief_values).unwrap_or_default()),
        network_contagion_score: Set(contagion_score),
    };
    let saved = assessment.insert(&state.db).await?;

    if !matches!(loan.status, LoanStatus::Closed | LoanStatus::ChargedOff | LoanStatus::Pending) {
        let new_status = match entity_state {
            RiskState::Current | RiskState::Watch => LoanStatus::Active,
            RiskState::Delinquent => LoanStatus::Delinquent,
            RiskState::Default => LoanStatus::Default,
        };
        if new_status != loan.status {
            let mut active: entity::loan::ActiveModel = loan.into();
            active.status = Set(new_status);
            active.updated_at = Set(Utc::now().into());
            active.update(&state.db).await?;
        }
    }

    Ok(saved)
}
