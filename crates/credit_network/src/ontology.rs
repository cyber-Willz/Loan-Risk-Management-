use std::collections::{HashMap, HashSet, VecDeque};

use ontology_engine::prelude::*;
use uuid::Uuid;

use crate::error::{CreditNetworkError, Result};
use crate::types::{BorrowerNode, RelationKind, RelationshipEdge};

const BORROWER_TYPE: &str = "Borrower";

/// Live, in-memory relationship graph over borrowers, backed by
/// `ontology_engine`. Complements the spectral hypergraph (which answers
/// "how risky/central is this borrower's neighborhood") with fast
/// object/link traversal (which answers "who exactly is connected to this
/// borrower, and how"). Rebuilt from the DB on each request in this
/// implementation — see module docs in `lib.rs` for the tradeoff.
pub struct RelationshipGraph {
    engine: OntologyEngine,
}

impl RelationshipGraph {
    /// Builds a fresh graph, registering the `Borrower` object type and one
    /// `LinkType` per `RelationKind`, then loading all nodes/edges.
    pub fn build(nodes: &[BorrowerNode], edges: &[RelationshipEdge]) -> Result<Self> {
        let engine = OntologyEngine::new();

        let borrower_type = ObjectTypeBuilder::new(BORROWER_TYPE)
            .primary_key("id")
            .property("id", PropertyType::String)
            .property("risk_score", PropertyType::Float)
            .build()
            .map_err(CreditNetworkError::Ontology)?;
        engine
            .register_object_type(borrower_type)
            .map_err(|e| CreditNetworkError::Ontology(e.to_string()))?;

        for kind in RelationKind::ALL {
            engine
                .register_link_type(LinkType::new(kind.as_str(), BORROWER_TYPE, BORROWER_TYPE))
                .map_err(|e| CreditNetworkError::Ontology(e.to_string()))?;
        }

        for node in nodes {
            let props = HashMap::from([
                ("id".to_string(), PropertyValue::String(node.id.to_string())),
                ("risk_score".to_string(), PropertyValue::Float(node.risk_score)),
            ]);
            engine
                .create_object_instance(ObjectInstance::new(node.id.to_string(), BORROWER_TYPE, props))
                .map_err(|e| CreditNetworkError::Ontology(e.to_string()))?;
        }

        for edge in edges {
            // Both directions so `related_borrowers` can traverse without
            // caring which side of the relationship a borrower was on
            // (e.g. "guarantor" naturally reads source->target, but a
            // borrower being *covered by* a guarantor should still surface
            // that guarantor as related).
            let link_type = edge.relation.as_str();
            let a = edge.source.to_string();
            let b = edge.target.to_string();
            if engine
                .create_link(LinkInstance::new(link_type, a.clone(), b.clone()))
                .is_err()
            {
                // Source/target may be missing from this snapshot (e.g. a
                // linked borrower outside the queried set) or the link may
                // already exist; either way this edge contributes nothing
                // further, so skip rather than fail the whole build.
                continue;
            }
            let _ = engine.create_link(LinkInstance::new(link_type, b, a));
        }

        Ok(Self { engine })
    }

    /// Breadth-first traversal of every registered relation type, up to
    /// `max_depth` hops, returning related borrower ids ordered by
    /// discovery (nearest first) and excluding `start` itself.
    pub fn related_borrowers(&self, start: Uuid, max_depth: usize) -> Vec<Uuid> {
        let start_id = start.to_string();
        let mut visited: HashSet<String> = HashSet::from([start_id.clone()]);
        let mut order: Vec<Uuid> = Vec::new();
        let mut frontier: VecDeque<(String, usize)> = VecDeque::from([(start_id, 0)]);

        while let Some((id, depth)) = frontier.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for kind in RelationKind::ALL {
                for neighbor in self.engine.traverse_link(&id, kind.as_str()) {
                    if visited.insert(neighbor.id.clone()) {
                        if let Ok(uuid) = Uuid::parse_str(&neighbor.id) {
                            order.push(uuid);
                        }
                        frontier.push_back((neighbor.id, depth + 1));
                    }
                }
            }
        }

        order
    }

    pub fn instance_count(&self) -> usize {
        self.engine.instance_count()
    }

    pub fn link_count(&self) -> usize {
        self.engine.link_count()
    }
}
