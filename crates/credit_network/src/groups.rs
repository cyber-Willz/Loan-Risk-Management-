use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::types::RelationshipEdge;

/// The result of folding `relationship_links` into shared-obligation
/// groups: edges with the same `(loan_id, relation)` become one
/// multi-member group (e.g. three co-borrowers on one loan), and
/// loan-independent edges (e.g. shared address) stay pairwise.
///
/// This is computed once, in plain `Uuid` space, and consumed by both
/// [`crate::graph::build_hypergraph`] (which needs it to build multi-way
/// hyperedges) and [`crate::analysis::analyze_contagion`] (which needs it
/// to compute each borrower's neighbor set for risk averaging) so the two
/// stay consistent -- a borrower's "neighbors" for risk-blending purposes
/// are exactly the same people they share a hyperedge with, not a looser
/// or stricter notion.
pub struct RelationshipGroups {
    /// Each shared-obligation group: at least 2 distinct borrower ids.
    pub groups: Vec<(Vec<Uuid>, f64)>,
    /// Loan-independent pairwise edges.
    pub pairwise: Vec<(Uuid, Uuid, f64)>,
}

impl RelationshipGroups {
    /// Builds the groups, restricted to edges whose both endpoints are in
    /// `known_ids` (so an edge referencing a borrower outside the current
    /// snapshot is dropped rather than corrupting a group), and dropping
    /// self-loops.
    pub fn build(edges: &[RelationshipEdge], known_ids: &HashSet<Uuid>) -> Self {
        let mut groups: HashMap<(Option<Uuid>, &'static str), (Vec<Uuid>, f64)> = HashMap::new();
        let mut pairwise = Vec::new();

        for edge in edges {
            if edge.source == edge.target {
                continue;
            }
            if !known_ids.contains(&edge.source) || !known_ids.contains(&edge.target) {
                continue;
            }

            match edge.loan_id {
                Some(loan_id) => {
                    let entry = groups
                        .entry((Some(loan_id), edge.relation.as_str()))
                        .or_insert_with(|| (Vec::new(), 0.0));
                    if !entry.0.contains(&edge.source) {
                        entry.0.push(edge.source);
                    }
                    if !entry.0.contains(&edge.target) {
                        entry.0.push(edge.target);
                    }
                    entry.1 = entry.1.max(edge.weight);
                }
                None => pairwise.push((edge.source, edge.target, edge.weight)),
            }
        }

        Self {
            groups: groups.into_values().filter(|(members, _)| members.len() >= 2).collect(),
            pairwise,
        }
    }

    /// Every borrower id that appears in at least one group or pairwise
    /// edge -- i.e. has at least one relationship at all.
    pub fn participant_ids(&self) -> HashSet<Uuid> {
        let mut ids = HashSet::new();
        for (members, _) in &self.groups {
            ids.extend(members.iter().copied());
        }
        for &(a, b, _) in &self.pairwise {
            ids.insert(a);
            ids.insert(b);
        }
        ids
    }

    /// Per-borrower neighbor sets: everyone sharing a group with them, plus
    /// direct pairwise partners.
    pub fn neighbor_sets(&self) -> HashMap<Uuid, HashSet<Uuid>> {
        let mut neighbors: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
        for (members, _) in &self.groups {
            for &a in members {
                for &b in members {
                    if a != b {
                        neighbors.entry(a).or_default().insert(b);
                    }
                }
            }
        }
        for &(a, b, _) in &self.pairwise {
            neighbors.entry(a).or_default().insert(b);
            neighbors.entry(b).or_default().insert(a);
        }
        neighbors
    }
}
