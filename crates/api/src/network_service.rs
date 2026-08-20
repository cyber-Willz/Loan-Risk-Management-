use std::collections::{HashMap, HashSet};

use chrono::Utc;
use credit_network::{analyze_contagion, BorrowerNode, ContagionResult, RelationKind, RelationshipEdge, RelationshipGraph};
use entity::prelude::*;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use uuid::Uuid;

use crate::dto::AnalyzeNetworkRequest;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Default number of clusters when the caller doesn't specify one.
const DEFAULT_CLUSTERS: usize = 4;

fn to_relation_kind(rt: RelationType) -> RelationKind {
    match rt {
        RelationType::Guarantor => RelationKind::Guarantor,
        RelationType::CoBorrower => RelationKind::CoBorrower,
        RelationType::SharedCollateral => RelationKind::SharedCollateral,
        RelationType::SharedEmployer => RelationKind::SharedEmployer,
        RelationType::SharedAddress => RelationKind::SharedAddress,
        RelationType::RelatedParty => RelationKind::RelatedParty,
    }
}

/// A borrower's network risk input: blends the belief mass on the three
/// non-`Current` states from their single worst (highest-risk) loan's most
/// recent assessment. Borrowers with no assessment yet contribute 0.0
/// (neutral) rather than being excluded, so newly onboarded related
/// parties still show up in the network.
async fn borrower_risk_scores(db: &sea_orm::DatabaseConnection, ids: &[Uuid]) -> ApiResult<HashMap<Uuid, f64>> {
    let mut scores = HashMap::with_capacity(ids.len());
    for &id in ids {
        let loans = Loan::find()
            .filter(entity::loan::Column::BorrowerId.eq(id))
            .all(db)
            .await?;
        let mut worst = 0.0_f64;
        for loan in loans {
            if let Some(assessment) = RiskAssessment::find()
                .filter(entity::risk_assessment::Column::LoanId.eq(loan.id))
                .order_by_desc(entity::risk_assessment::Column::AssessedAt)
                .one(db)
                .await?
            {
                let belief: Vec<f64> = serde_json::from_value(assessment.belief).unwrap_or_default();
                if belief.len() == 4 {
                    let risk = belief[1] * 0.33 + belief[2] * 0.66 + belief[3] * 1.0;
                    worst = worst.max(risk);
                }
            }
        }
        scores.insert(id, worst);
    }
    Ok(scores)
}

async fn load_edges(db: &sea_orm::DatabaseConnection, ids: Option<&[Uuid]>) -> ApiResult<Vec<RelationshipLinkModel>> {
    let mut query = RelationshipLink::find();
    if let Some(ids) = ids {
        let ids = ids.to_vec();
        query = query.filter(
            entity::relationship_link::Column::SourceBorrowerId
                .is_in(ids.clone())
                .or(entity::relationship_link::Column::TargetBorrowerId.is_in(ids)),
        );
    }
    Ok(query.all(db).await?)
}

fn to_network_edges(links: &[RelationshipLinkModel]) -> Vec<RelationshipEdge> {
    links
        .iter()
        .map(|l| RelationshipEdge {
            source: l.source_borrower_id,
            target: l.target_borrower_id,
            relation: to_relation_kind(l.relation_type),
            loan_id: l.loan_id,
            weight: l.weight,
        })
        .collect()
}

/// Runs a spectral contagion analysis and persists the results as a new
/// `network_snapshots` batch. When `req.borrower_ids` is set, the borrower
/// set is exactly the union of those ids and everyone directly linked to
/// them; otherwise every borrower with at least one relationship link is
/// included.
pub async fn analyze(state: &AppState, req: AnalyzeNetworkRequest) -> ApiResult<Vec<ContagionResult>> {
    let links = load_edges(&state.db, req.borrower_ids.as_deref()).await?;

    let mut borrower_ids: HashSet<Uuid> = HashSet::new();
    if let Some(seed) = &req.borrower_ids {
        borrower_ids.extend(seed.iter().copied());
    }
    for link in &links {
        borrower_ids.insert(link.source_borrower_id);
        borrower_ids.insert(link.target_borrower_id);
    }
    let borrower_ids: Vec<Uuid> = borrower_ids.into_iter().collect();

    if borrower_ids.len() < 2 {
        return Err(ApiError::Network(credit_network::CreditNetworkError::TooFewBorrowers(
            borrower_ids.len(),
        )));
    }

    let risk_scores = borrower_risk_scores(&state.db, &borrower_ids).await?;
    let nodes: Vec<BorrowerNode> = borrower_ids
        .iter()
        .map(|&id| BorrowerNode {
            id,
            risk_score: risk_scores.get(&id).copied().unwrap_or(0.0),
        })
        .collect();
    let edges = to_network_edges(&links);

    let k = req.k_clusters.unwrap_or(DEFAULT_CLUSTERS.min(nodes.len().max(1)));
    let results = analyze_contagion(&nodes, &edges, k)?;

    let snapshot_id = Uuid::new_v4();
    let now = Utc::now();
    for result in &results {
        let row = entity::network_snapshot::ActiveModel {
            id: Set(Uuid::new_v4()),
            snapshot_id: Set(snapshot_id),
            borrower_id: Set(result.borrower_id),
            computed_at: Set(now.into()),
            cluster_id: Set(result.cluster_id as i32),
            fiedler_component: Set(result.fiedler_component),
            contagion_score: Set(result.contagion_score),
            degree: Set(result.degree as i32),
        };
        row.insert(&state.db).await?;
    }

    Ok(results)
}

/// Loads every borrower and relationship link, builds a live traversal
/// graph, and returns everyone within `max_depth` hops of `borrower_id`.
/// Rebuilding the whole graph per request is the right tradeoff for a
/// lending book at moderate scale (thousands, not millions, of borrowers);
/// see `credit_network::ontology` module docs for the incremental
/// alternative if this ever becomes a hot path.
pub async fn related_borrowers(state: &AppState, borrower_id: Uuid, max_depth: usize) -> ApiResult<Vec<BorrowerModel>> {
    let all_borrowers = Borrower::find().all(&state.db).await?;
    let all_links = RelationshipLink::find().all(&state.db).await?;

    let ids: Vec<Uuid> = all_borrowers.iter().map(|b| b.id).collect();
    let risk_scores = borrower_risk_scores(&state.db, &ids).await?;
    let nodes: Vec<BorrowerNode> = ids
        .iter()
        .map(|&id| BorrowerNode {
            id,
            risk_score: risk_scores.get(&id).copied().unwrap_or(0.0),
        })
        .collect();
    let edges = to_network_edges(&all_links);

    let graph = RelationshipGraph::build(&nodes, &edges)?;
    let related_ids = graph.related_borrowers(borrower_id, max_depth);

    let related_set: HashSet<Uuid> = related_ids.into_iter().collect();
    Ok(all_borrowers.into_iter().filter(|b| related_set.contains(&b.id)).collect())
}
