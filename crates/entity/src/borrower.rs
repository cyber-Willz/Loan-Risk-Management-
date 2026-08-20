use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, EnumIter, DeriveActiveEnum, PartialEq, Eq, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(32))")]
#[serde(rename_all = "snake_case")]
pub enum BorrowerType {
    #[sea_orm(string_value = "individual")]
    Individual,
    #[sea_orm(string_value = "company")]
    Company,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "borrowers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub borrower_type: BorrowerType,
    /// National ID / company registration number. Not unique-enforced at the
    /// DB level because sanitized/partial IDs from legacy intake can collide;
    /// network analytics treats a shared national_id as a strong relationship
    /// signal rather than the DB treating it as an identity key.
    pub national_id: Option<String>,
    pub employer: Option<String>,
    pub address: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::loan::Entity")]
    Loan,
}

impl Related<super::loan::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Loan.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
