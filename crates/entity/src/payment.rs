use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, EnumIter, DeriveActiveEnum, PartialEq, Eq, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(32))")]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    #[sea_orm(string_value = "scheduled")]
    Scheduled,
    #[sea_orm(string_value = "paid_on_time")]
    PaidOnTime,
    #[sea_orm(string_value = "paid_late")]
    PaidLate,
    #[sea_orm(string_value = "missed")]
    Missed,
    #[sea_orm(string_value = "partial")]
    Partial,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "payments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub loan_id: Uuid,
    pub due_date: Date,
    pub paid_date: Option<Date>,
    pub amount_due: Decimal,
    pub amount_paid: Decimal,
    pub status: PaymentStatus,
    /// Positive = days late, 0 = on time or early, unset until due_date has
    /// passed or payment has posted.
    pub days_late: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::loan::Entity",
        from = "Column::LoanId",
        to = "super::loan::Column::Id"
    )]
    Loan,
}

impl Related<super::loan::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Loan.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
