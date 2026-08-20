use std::collections::HashSet;

use spectral_hypergraph::spectral::{fiedler_vector, spectral_cluster};

use crate::error::{CreditNetworkError, Result};
use crate::graph::build_hypergraph;
use crate::groups::RelationshipGroups;
use crate::types::{BorrowerNode, ContagionResult, RelationshipEdge};

/// Weight given to a borrower's own risk score in the contagion blend.
const OWN_RISK_WEIGHT: f64 = 0.55;
/// Weight given to the mean risk score of a borrower's direct neighbors.
const NEIGHBOR_RISK_WEIGHT: f64 = 0.30;
/// Weight given to structural centrality (normalized |Fiedler component|).
const CENTRALITY_WEIGHT: f64 = 0.15;

/// Runs spectral clustering + Fiedler centrality over the borrower
/// relationship network and blends each borrower's own risk with its
/// neighbors' risk and its structural position to produce a contagion
/// score in `[0.0, 1.0]`.
///
/// Borrowers with no relationships at all ("isolated") are excluded from
/// the spectral computation itself (a degree-0 vertex has no structural
/// position to measure) and instead get their own `risk_score` back
/// unchanged, with `cluster_id = 0`, `fiedler_component = 0.0`, and
/// `degree = 0` -- they're still included in the returned results, just
/// network-neutral.
///
/// `k_clusters` is clamped to `[1, num_networked_borrowers]`. Returns
/// [`CreditNetworkError::TooFewBorrowers`] if fewer than 2 borrowers have
/// any relationship at all (nothing to analyze).
pub fn analyze_contagion(
    nodes: &[BorrowerNode],
    edges: &[RelationshipEdge],
    k_clusters: usize,
) -> Result<Vec<ContagionResult>> {
    let known_ids: HashSet<_> = nodes.iter().map(|n| n.id).collect();
    let relationship_groups = RelationshipGroups::build(edges, &known_ids);
    let participants = relationship_groups.participant_ids();

    let (networked, isolated): (Vec<_>, Vec<_>) = nodes.iter().partition(|n| participants.contains(&n.id));

    if networked.len() < 2 {
        return Err(CreditNetworkError::TooFewBorrowers(networked.len()));
    }
    let k = k_clusters.clamp(1, networked.len());

    let networked_nodes: Vec<BorrowerNode> = networked.into_iter().cloned().collect();
    let (hg, index) = build_hypergraph(&networked_nodes, edges)?;

    let fiedler = fiedler_vector(&hg)?;
    let clusters = spectral_cluster(&hg, k, hg.num_vertices() > 500, 0xC0FFEE)?;
    let neighbors = relationship_groups.neighbor_sets();

    let risk_by_id: std::collections::HashMap<_, _> = nodes.iter().map(|n| (n.id, n.risk_score)).collect();

    // Min-max normalize |fiedler| across vertices so the centrality term is
    // comparable across networks of very different scale/connectivity.
    let abs_fiedler: Vec<f64> = fiedler.iter().map(|v| v.abs()).collect();
    let max_abs = abs_fiedler.iter().cloned().fold(0.0_f64, f64::max);

    let mut results = Vec::with_capacity(nodes.len());
    for node in &networked_nodes {
        let Some(&vid) = index.borrower_to_vertex.get(&node.id) else {
            continue;
        };
        let node_neighbors = neighbors.get(&node.id);
        let degree = node_neighbors.map(HashSet::len).unwrap_or(0);
        let neighbor_risk = node_neighbors
            .filter(|n| !n.is_empty())
            .map(|n| n.iter().filter_map(|id| risk_by_id.get(id)).sum::<f64>() / n.len() as f64)
            .unwrap_or(node.risk_score); // no neighbors: fall back to own risk rather than zeroing the term

        let fiedler_component = fiedler.get(vid.0).copied().unwrap_or(0.0);
        let centrality = if max_abs > 1e-12 {
            fiedler_component.abs() / max_abs
        } else {
            0.0
        };

        let contagion_score = (OWN_RISK_WEIGHT * node.risk_score
            + NEIGHBOR_RISK_WEIGHT * neighbor_risk
            + CENTRALITY_WEIGHT * centrality)
            .clamp(0.0, 1.0);

        results.push(ContagionResult {
            borrower_id: node.id,
            cluster_id: clusters.assignments.get(vid.0).copied().unwrap_or(0),
            fiedler_component,
            contagion_score,
            degree,
        });
    }

    for node in isolated {
        results.push(ContagionResult {
            borrower_id: node.id,
            cluster_id: 0,
            fiedler_component: 0.0,
            contagion_score: node.risk_score,
            degree: 0,
        });
    }

    Ok(results)
}
