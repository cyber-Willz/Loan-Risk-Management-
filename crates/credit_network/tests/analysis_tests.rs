use credit_network::{analyze_contagion, BorrowerNode, CreditNetworkError, RelationKind, RelationshipEdge, RelationshipGraph};
use uuid::Uuid;

fn node(risk: f64) -> (Uuid, BorrowerNode) {
    let id = Uuid::new_v4();
    (id, BorrowerNode { id, risk_score: risk })
}

/// A risky borrower's low-risk co-borrower should end up with a higher
/// contagion score than an isolated low-risk borrower with no
/// relationships at all -- that's the whole point of the network layer.
#[test]
fn contagion_propagates_from_risky_neighbor() {
    let (risky_id, risky) = node(0.95);
    let (linked_id, linked) = node(0.05);
    let (isolated_id, isolated) = node(0.05);

    let nodes = vec![risky.clone(), linked.clone(), isolated.clone()];
    let loan_id = Uuid::new_v4();
    let edges = vec![RelationshipEdge {
        source: risky_id,
        target: linked_id,
        relation: RelationKind::CoBorrower,
        loan_id: Some(loan_id),
        weight: 1.0,
    }];

    let results = analyze_contagion(&nodes, &edges, 2).expect("analysis should succeed with 3 borrowers");
    assert_eq!(results.len(), 3);

    let linked_score = results.iter().find(|r| r.borrower_id == linked_id).unwrap().contagion_score;
    let isolated_score = results.iter().find(|r| r.borrower_id == isolated_id).unwrap().contagion_score;

    assert!(
        linked_score > isolated_score,
        "co-borrower of a risky loan ({linked_score}) should score higher than an unrelated borrower ({isolated_score})"
    );
    // isolated borrower has no edges, so contagion falls back to their own risk score
    assert!((isolated_score - isolated.risk_score).abs() < 1e-9);
}

#[test]
fn rejects_analysis_with_fewer_than_two_borrowers() {
    let (id, only) = node(0.1);
    let err = analyze_contagion(&[only], &[], 1).unwrap_err();
    // a lone borrower with no edges has zero *networked* participants
    assert!(matches!(err, CreditNetworkError::TooFewBorrowers(0)));
    let _ = id;
}

#[test]
fn same_loan_relationships_fold_into_one_hyperedge() {
    // Three co-borrowers on the same loan should still produce a valid,
    // fully-connected analysis (i.e. the multi-way grouping in `graph.rs`
    // didn't silently drop anyone).
    let (a_id, a) = node(0.2);
    let (b_id, b) = node(0.3);
    let (c_id, c) = node(0.4);
    let loan_id = Uuid::new_v4();

    let nodes = vec![a, b, c];
    let edges = vec![
        RelationshipEdge {
            source: a_id,
            target: b_id,
            relation: RelationKind::CoBorrower,
            loan_id: Some(loan_id),
            weight: 1.0,
        },
        RelationshipEdge {
            source: b_id,
            target: c_id,
            relation: RelationKind::CoBorrower,
            loan_id: Some(loan_id),
            weight: 1.0,
        },
    ];

    let results = analyze_contagion(&nodes, &edges, 1).expect("three co-borrowers should analyze cleanly");
    assert_eq!(results.len(), 3);
    for r in &results {
        assert_eq!(r.degree, 2, "each of the three co-borrowers should see the other two as neighbors");
    }
}

#[test]
fn related_borrowers_traverses_multiple_hops() {
    let (a_id, a) = node(0.1);
    let (b_id, b) = node(0.1);
    let (c_id, c) = node(0.1);

    let nodes = vec![a, b, c];
    let edges = vec![
        RelationshipEdge {
            source: a_id,
            target: b_id,
            relation: RelationKind::Guarantor,
            loan_id: None,
            weight: 1.0,
        },
        RelationshipEdge {
            source: b_id,
            target: c_id,
            relation: RelationKind::SharedAddress,
            loan_id: None,
            weight: 1.0,
        },
    ];

    let graph = RelationshipGraph::build(&nodes, &edges).expect("graph should build");

    let one_hop = graph.related_borrowers(a_id, 1);
    assert_eq!(one_hop, vec![b_id], "depth 1 should only reach the direct guarantor link");

    let two_hop = graph.related_borrowers(a_id, 2);
    assert!(two_hop.contains(&b_id) && two_hop.contains(&c_id), "depth 2 should reach both b and c");
    assert!(!two_hop.contains(&a_id), "a borrower should never be listed as related to themselves");
}
