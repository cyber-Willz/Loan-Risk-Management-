//! Credit relationship network analytics for the loan risk system.
//!
//! Combines two complementary graph representations of the same
//! `relationship_links` data:
//!
//! * [`analysis::analyze_contagion`] builds a [`spectral_hypergraph`] over
//!   borrowers (grouping same-loan relationships into multi-way
//!   hyperedges) and computes Fiedler centrality + spectral clustering to
//!   produce a per-borrower contagion risk score — "how exposed is this
//!   borrower to risk elsewhere in the network".
//! * [`ontology::RelationshipGraph`] builds an `ontology_engine`
//!   object/link graph over the same data for fast BFS traversal —
//!   "exactly who is connected to this borrower, and how".
//!
//! Both are rebuilt from whatever `BorrowerNode`/`RelationshipEdge` slice
//! the caller supplies (typically loaded from `relationship_links` +
//! current risk assessments for one query), rather than kept as
//! long-lived mutable state. For this system's expected scale (a lending
//! book's relationship graph, recomputed on demand or on a schedule) that
//! keeps the analytics layer simple and side-effect-free; a high-frequency
//! variant could instead maintain a persistent `RelationshipGraph` updated
//! incrementally as links are created.

pub mod analysis;
pub mod error;
pub mod graph;
pub mod groups;
pub mod ontology;
pub mod types;

pub use analysis::analyze_contagion;
pub use error::{CreditNetworkError, Result};
pub use ontology::RelationshipGraph;
pub use types::{BorrowerNode, ContagionResult, RelationKind, RelationshipEdge};
