use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Mirrors `entity::relationship_link::RelationType` without pulling in
/// SeaORM's derive machinery — this crate only needs the discriminant to
/// pick edge weighting and ontology link-type names.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationKind {
    Guarantor,
    CoBorrower,
    SharedCollateral,
    SharedEmployer,
    SharedAddress,
    RelatedParty,
}

impl RelationKind {
    /// Stable name used both as the `ontology_engine` `LinkType` name and
    /// for logging; keep in sync with `entity::relationship_link::RelationType`'s
    /// `string_value`s.
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationKind::Guarantor => "guarantor",
            RelationKind::CoBorrower => "co_borrower",
            RelationKind::SharedCollateral => "shared_collateral",
            RelationKind::SharedEmployer => "shared_employer",
            RelationKind::SharedAddress => "shared_address",
            RelationKind::RelatedParty => "related_party",
        }
    }

    pub const ALL: [RelationKind; 6] = [
        RelationKind::Guarantor,
        RelationKind::CoBorrower,
        RelationKind::SharedCollateral,
        RelationKind::SharedEmployer,
        RelationKind::SharedAddress,
        RelationKind::RelatedParty,
    ];
}

/// A borrower as seen by the network layer: just enough to build the graph
/// and seed contagion propagation. `risk_score` should be in `[0.0, 1.0]`,
/// typically the borrower's current worst-loan default/delinquent belief
/// probability from `credit_risk_hmm_engine`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BorrowerNode {
    pub id: Uuid,
    pub risk_score: f64,
}

/// A relationship edge between two borrowers, as loaded from
/// `relationship_links`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationshipEdge {
    pub source: Uuid,
    pub target: Uuid,
    pub relation: RelationKind,
    /// When present, all edges sharing the same `(loan_id, relation)` pair
    /// are folded into a single multi-way hyperedge instead of a pairwise
    /// one — e.g. three co-borrowers on one loan become a single 3-vertex
    /// hyperedge rather than three separate pairwise edges, which better
    /// reflects that they share one obligation.
    pub loan_id: Option<Uuid>,
    pub weight: f64,
}

/// Per-borrower output of a network analysis run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContagionResult {
    pub borrower_id: Uuid,
    pub cluster_id: usize,
    pub fiedler_component: f64,
    pub contagion_score: f64,
    pub degree: usize,
}
