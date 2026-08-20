use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Edge type in the credit relationship network. Each variant maps to a
/// hyperedge or pairwise edge in `credit_network`'s spectral hypergraph
/// construction, and to a `LinkType` in the `ontology_engine` live graph.
#[derive(Copy, Clone, Debug, EnumIter, DeriveActiveEnum, PartialEq, Eq, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(32))")]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    #[sea_orm(string_value = "guarantor")]
    Guarantor,
    #[sea_orm(string_value = "co_borrower")]
    CoBorrower,
    #[sea_orm(string_value = "shared_collateral")]
    SharedCollateral,
    #[sea_orm(string_value = "shared_employer")]
    SharedEmployer,
    #[sea_orm(string_value = "shared_address")]
    SharedAddress,
    #[sea_orm(string_value = "related_party")]
    RelatedParty,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "relationship_links")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub source_borrower_id: Uuid,
    pub target_borrower_id: Uuid,
    pub relation_type: RelationType,
    /// Optional loan this link is scoped to (e.g. a specific guarantee).
    /// Null means the relationship is loan-independent (e.g. shared address).
    pub loan_id: Option<Uuid>,
    /// Edge/incidence weight used directly as the hyperedge weight in
    /// `spectral_hypergraph`; defaults to 1.0 and can be tuned per relation
    /// type strength.
    pub weight: f64,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::borrower::Entity",
        from = "Column::SourceBorrowerId",
        to = "super::borrower::Column::Id"
    )]
    SourceBorrower,
}

impl ActiveModelBehavior for ActiveModel {}
