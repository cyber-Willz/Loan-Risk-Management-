use std::collections::{HashMap, HashSet};

use spectral_hypergraph::hypergraph::{HypergraphBuilder, SpectralHypergraph, VertexId};
use uuid::Uuid;

use crate::error::Result;
use crate::groups::RelationshipGroups;
use crate::types::{BorrowerNode, RelationshipEdge};

/// Bidirectional mapping between borrower ids and their hypergraph vertex,
/// needed because `spectral_hypergraph` only knows about opaque
/// `VertexId`/string-label pairs, not our domain types.
pub struct VertexIndex {
    pub borrower_to_vertex: HashMap<Uuid, VertexId>,
    pub vertex_to_borrower: HashMap<VertexId, Uuid>,
}

impl VertexIndex {
    pub fn borrower_of(&self, v: VertexId) -> Option<Uuid> {
        self.vertex_to_borrower.get(&v).copied()
    }
}

/// Builds a [`SpectralHypergraph`] from borrowers and their relationship
/// edges, using the shared-obligation grouping from [`RelationshipGroups`]
/// (see its docs for why co-obligors fold into one multi-way hyperedge
/// instead of pairwise ones).
///
/// Only borrowers that appear in at least one relationship become
/// vertices: `spectral_hypergraph` rejects isolated (degree-0) vertices
/// outright, and a borrower with no relationships has no network position
/// to analyze anyway -- callers should treat such borrowers as
/// network-neutral (contagion score == their own risk) rather than
/// passing them in here. See [`crate::analysis::analyze_contagion`],
/// which does exactly that.
pub fn build_hypergraph(
    nodes: &[BorrowerNode],
    edges: &[RelationshipEdge],
) -> Result<(SpectralHypergraph, VertexIndex)> {
    let known_ids: HashSet<Uuid> = nodes.iter().map(|n| n.id).collect();
    let relationship_groups = RelationshipGroups::build(edges, &known_ids);
    let participants = relationship_groups.participant_ids();

    let mut builder = HypergraphBuilder::with_capacity(participants.len(), edges.len(), 2);
    let mut borrower_to_vertex = HashMap::with_capacity(participants.len());
    let mut vertex_to_borrower = HashMap::with_capacity(participants.len());

    for node in nodes {
        if !participants.contains(&node.id) {
            continue;
        }
        let vid = builder.get_or_add_vertex(node.id.to_string())?;
        borrower_to_vertex.insert(node.id, vid);
        vertex_to_borrower.insert(vid, node.id);
    }

    for (members, weight) in &relationship_groups.groups {
        let vids: Vec<VertexId> = members.iter().filter_map(|id| borrower_to_vertex.get(id)).copied().collect();
        if vids.len() >= 2 {
            builder.add_hyperedge(&vids, *weight)?;
        }
    }
    for &(a, b, weight) in &relationship_groups.pairwise {
        if let (Some(&sv), Some(&tv)) = (borrower_to_vertex.get(&a), borrower_to_vertex.get(&b)) {
            builder.add_hyperedge(&[sv, tv], weight)?;
        }
    }

    let hg = builder.build()?;
    Ok((
        hg,
        VertexIndex {
            borrower_to_vertex,
            vertex_to_borrower,
        },
    ))
}
