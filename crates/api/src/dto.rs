use chrono::NaiveDate;
use entity::prelude::*;
use sea_orm::prelude::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---- Borrowers ----------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateBorrowerRequest {
    pub name: String,
    pub borrower_type: BorrowerType,
    pub national_id: Option<String>,
    pub employer: Option<String>,
    pub address: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

// ---- Loans ---------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateLoanRequest {
    pub borrower_id: Uuid,
    pub principal_amount: Decimal,
    pub interest_rate_bps: i32,
    pub term_months: i32,
    pub origination_date: NaiveDate,
    pub collateral_value: Option<Decimal>,
    pub purpose: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListLoansQuery {
    pub borrower_id: Option<Uuid>,
    pub status: Option<LoanStatus>,
}

// ---- Payments --------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RecordPaymentRequest {
    pub due_date: NaiveDate,
    pub amount_due: Decimal,
    /// Omit for a still-outstanding scheduled payment; provide once cash
    /// has actually been received.
    pub paid_date: Option<NaiveDate>,
    #[serde(default)]
    pub amount_paid: Decimal,
}

// ---- Relationships ---------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateRelationshipRequest {
    pub source_borrower_id: Uuid,
    pub target_borrower_id: Uuid,
    pub relation_type: RelationType,
    pub loan_id: Option<Uuid>,
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 {
    1.0
}

// ---- Risk ------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct RiskAssessmentResponse {
    pub loan_id: Uuid,
    pub state: RiskState,
    pub state_probability: f64,
    pub belief: Vec<f64>,
    pub network_contagion_score: f64,
    pub assessed_at: chrono::DateTime<chrono::FixedOffset>,
}

impl From<RiskAssessmentModel> for RiskAssessmentResponse {
    fn from(m: RiskAssessmentModel) -> Self {
        let belief = serde_json::from_value(m.belief).unwrap_or_default();
        Self {
            loan_id: m.loan_id,
            state: m.state,
            state_probability: m.state_probability,
            belief,
            network_contagion_score: m.network_contagion_score,
            assessed_at: m.assessed_at,
        }
    }
}

// ---- Network -----------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AnalyzeNetworkRequest {
    /// Restrict analysis to this set of borrowers (and everyone they're
    /// directly linked to for context). Omit to analyze every borrower
    /// with at least one relationship link.
    pub borrower_ids: Option<Vec<Uuid>>,
    pub k_clusters: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ContagionResultResponse {
    pub borrower_id: Uuid,
    pub cluster_id: usize,
    pub fiedler_component: f64,
    pub contagion_score: f64,
    pub degree: usize,
}

impl From<credit_network::ContagionResult> for ContagionResultResponse {
    fn from(r: credit_network::ContagionResult) -> Self {
        Self {
            borrower_id: r.borrower_id,
            cluster_id: r.cluster_id,
            fiedler_component: r.fiedler_component,
            contagion_score: r.contagion_score,
            degree: r.degree,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RelatedBorrowersResponse {
    pub borrower_id: Uuid,
    pub related: Vec<BorrowerModel>,
}
