use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, EnumIter, DeriveActiveEnum, PartialEq, Eq, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(32))")]
#[serde(rename_all = "snake_case")]
pub enum LoanStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "delinquent")]
    Delinquent,
    #[sea_orm(string_value = "default")]
    Default,
    #[sea_orm(string_value = "closed")]
    Closed,
    #[sea_orm(string_value = "charged_off")]
    ChargedOff,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "loans")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub borrower_id: Uuid,
    /// Principal amount in the loan's minor currency unit (e.g. cents) to
    /// avoid floating point drift in balances.
    pub principal_amount: Decimal,
    pub interest_rate_bps: i32,
    pub term_months: i32,
    pub status: LoanStatus,
    pub origination_date: Date,
    pub maturity_date: Date,
    pub collateral_value: Option<Decimal>,
    pub purpose: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::borrower::Entity",
        from = "Column::BorrowerId",
        to = "super::borrower::Column::Id"
    )]
    Borrower,
    #[sea_orm(has_many = "super::payment::Entity")]
    Payment,
    #[sea_orm(has_many = "super::risk_assessment::Entity")]
    RiskAssessment,
}

impl Related<super::borrower::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Borrower.def()
    }
}

impl Related<super::payment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Payment.def()
    }
}

impl Related<super::risk_assessment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RiskAssessment.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
