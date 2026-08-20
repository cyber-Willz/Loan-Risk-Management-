use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, EnumIter, DeriveActiveEnum, PartialEq, Eq, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(32))")]
#[serde(rename_all = "snake_case")]
pub enum RiskState {
    #[sea_orm(string_value = "current")]
    Current,
    #[sea_orm(string_value = "watch")]
    Watch,
    #[sea_orm(string_value = "delinquent")]
    Delinquent,
    #[sea_orm(string_value = "default")]
    Default,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "risk_assessments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub loan_id: Uuid,
    pub assessed_at: DateTimeWithTimeZone,
    /// Most probable hidden state from the Neural HMM belief vector.
    pub state: RiskState,
    /// Probability mass on `state` (belief.argmax().1).
    pub state_probability: f64,
    /// Full belief distribution over [Current, Watch, Delinquent, Default],
    /// serialized so the state transition history stays inspectable without
    /// re-running the filter.
    pub belief: Json,
    /// Contagion score contributed by the credit network at assessment time
    /// (from `credit_network`'s spectral analysis), 0.0 if not networked.
    pub network_contagion_score: f64,
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
