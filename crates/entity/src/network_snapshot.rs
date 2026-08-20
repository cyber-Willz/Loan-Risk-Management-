use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One row per borrower per computed network snapshot, storing the result
/// of a `credit_network` spectral analysis run over the whole relationship
/// graph at that point in time.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "network_snapshots")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Groups all rows produced by a single analytics run.
    pub snapshot_id: Uuid,
    pub borrower_id: Uuid,
    pub computed_at: DateTimeWithTimeZone,
    /// Spectral cluster assignment (k-means on the Laplacian embedding).
    pub cluster_id: i32,
    /// Fiedler (algebraic connectivity) vector component for this borrower;
    /// large magnitude indicates a structurally important bridging position
    /// in the relationship graph.
    pub fiedler_component: f64,
    /// Aggregate contagion risk score: cluster-weighted blend of this
    /// borrower's own risk state and its neighbors' risk states.
    pub contagion_score: f64,
    /// Degree (number of relationship edges) at snapshot time.
    pub degree: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
