# Loan Risk Management & Credit Network Analytics System

An Axum + SeaORM/PostgreSQL backend for loan servicing, HMM-based credit
risk scoring, and hypergraph-based credit network (contagion) analytics.
Built around three integrated engines:

- **`credit_risk_hmm_engine`** (your `neural_hmm` crate, vendored as-is) —
  a Burn-based neural hidden Markov model.
- **`spectral_hypergraph`** (vendored as-is) — Laplacian/Fiedler spectral
  graph analysis over hypergraphs.
- **`ontology_engine`** (vendored as-is) — a typed object/link graph
  engine, used here for fast related-party traversal.

Everything else in `crates/` is new: the domain schema, the HMM →
loan-risk wrapper, the hypergraph → credit-network wrapper, and the Axum
API tying it together.

## Architecture

```
crates/
  entity/                  SeaORM entities (borrowers, loans, payments,
                            relationship_links, risk_assessments,
                            network_snapshots)
  migration/                schema migration + a standalone up/down/fresh
                            runner (see "Why not sea-orm-cli" below)

  credit_risk_hmm_engine/  vendored `neural_hmm` — untouched
  credit_risk/             loan-risk domain wrapper around it:
                              - 4-state space: Current / Watch / Delinquent / Default
                              - PaymentFeatures: 8-dim observation vector
                              - hand-specified transition matrix
                              - CreditRiskFilter (NOT Sync -- see below)

  spectral_hypergraph/     vendored -- untouched
  ontology_engine/         vendored -- untouched
  credit_network/          glue crate:
                              - graph.rs: builds a SpectralHypergraph from
                                relationship_links, folding same-loan
                                relationships (e.g. 3 co-borrowers on one
                                loan) into a single multi-way hyperedge
                                instead of pairwise edges
                              - groups.rs: the shared grouping logic behind
                                both the hyperedges and neighbor-risk
                                averaging, so the two stay consistent
                              - analysis.rs: Fiedler centrality + spectral
                                clustering -> per-borrower contagion score
                              - ontology.rs: ontology_engine-backed BFS for
                                "who is related to this borrower"

  api/                      Axum server: routes, DTOs, domain services
                              - risk_actor.rs: dedicated-thread actor that
                                owns the CreditRiskFilter (see below)
```

### Why a "risk actor" thread instead of `Arc<CreditRiskFilter>`

Burn 0.13's `Param<Tensor<_>>` holds a `Box<dyn Fn(..) -> Tensor + Send>`
in its lazy-init slot, which has no `Sync` bound. That makes
`CreditRiskFilter` (and anything containing a Burn `Module`) `Send` but
**never `Sync`**, regardless of feature flags — a real limitation in that
Burn version, not something fixable from this codebase. Axum's shared
`State` requires `Sync`.

The fix (`api/src/risk_actor.rs`): exactly one OS thread owns the
`CreditRiskFilter`. `AppState` holds a `RiskActorHandle` instead — a
cheap `Clone + Send + Sync` wrapper around an `mpsc` channel that only
ever moves plain `Belief`/`PaymentFeatures` data (never tensors) across
the boundary. Every `/loans/:id/risk/assess` call round-trips through
that channel.

### Why the migration crate has its own CLI instead of `sea-orm-cli`

`sea-orm-migration`'s `cli` feature pulls in `sea-orm-cli`, which does not
build on this workspace's pinned rustc 1.75 toolchain (an internal
`regex`/`std::error::Error` trait-bound mismatch in `sea-orm-cli 0.12.15`,
unrelated to this project's code). `migration/src/main.rs` is a ~20-line
replacement using `MigratorTrait` directly: `up`, `down`, `fresh`,
`refresh`, `status`.

## Requirements

- Rust **1.75** (the workspace is pinned to this MSRV — see "Dependency
  pins" below). Install via `apt-get install cargo rustc` on
  Debian/Ubuntu, or `rustup install 1.75.0`.
- PostgreSQL 13+ reachable via `DATABASE_URL`.

## Running it

```bash
cp .env.example .env      # edit DATABASE_URL if needed
createdb loan_risk        # or let the server's own connection create it
                           # if your Postgres role has CREATEDB

cargo run -p api           # runs migrations automatically, then listens
                           # on BIND_ADDR (default 0.0.0.0:8080)
```

To manage migrations separately from the server:

```bash
cargo run -p migration -- up       # apply all pending migrations
cargo run -p migration -- status   # show applied/pending
cargo run -p migration -- down     # revert the last migration
cargo run -p migration -- fresh    # drop everything and reapply
```

Run with `SKIP_MIGRATIONS=1 cargo run -p api` to skip the automatic
migration step (e.g. if a deploy pipeline runs migrations as a separate
step).

## API surface

All endpoints accept/return JSON. Enum fields (`borrower_type`,
`status`, `relation_type`, `state`, ...) use lowercase snake_case values
matching the stored DB strings, e.g. `"individual"`, `"co_borrower"`,
`"delinquent"`.

### Borrowers
| Method | Path | Notes |
|---|---|---|
| POST | `/borrowers` | create |
| GET | `/borrowers` | list all |
| GET | `/borrowers/:id` | fetch one |
| GET | `/borrowers/:id/relationships` | direct relationship links (either direction) |
| GET | `/borrowers/:id/related?depth=N` | BFS over the whole relationship graph (default depth 2) |

### Loans
| Method | Path | Notes |
|---|---|---|
| POST | `/loans` | create; `maturity_date` is derived from `origination_date + term_months` |
| GET | `/loans?borrower_id=&status=` | list, both filters optional |
| GET | `/loans/:id` | fetch one |

### Payments
| Method | Path | Notes |
|---|---|---|
| POST | `/loans/:loan_id/payments` | record a scheduled/actual payment; `status`/`days_late` are derived server-side |
| GET | `/loans/:loan_id/payments` | list, most recent due date first |

### Relationships
| Method | Path | Notes |
|---|---|---|
| POST | `/relationships` | link two borrowers (`guarantor`, `co_borrower`, `shared_collateral`, `shared_employer`, `shared_address`, `related_party`), optionally scoped to a `loan_id` |

### Risk assessment
| Method | Path | Notes |
|---|---|---|
| POST | `/loans/:id/risk/assess` | runs one HMM filter step from the prior belief (or uniform, if none), persists the result, and nudges `loan.status` to match (unless the loan is `closed`/`charged_off`/`pending`) |
| GET | `/loans/:id/risk` | latest assessment, or `null` |
| GET | `/loans/:id/risk/history` | full history, most recent first |

### Network analytics
| Method | Path | Notes |
|---|---|---|
| POST | `/network/analyze` | body: `{ "borrower_ids": [...] | null, "k_clusters": N | null }`. Omit `borrower_ids` to analyze every borrower with at least one relationship. Persists a new `network_snapshots` batch and returns per-borrower results. |

## Important caveats (read before relying on this in production)

1. **The HMM emission network is untrained.** `CreditRiskFilter::new()`
   builds the network with freshly-initialized (random) weights — there's
   no labeled roll-rate dataset bundled with this system. The transition
   matrix in `credit_risk/src/filter.rs` is hand-specified from standard
   servicing intuition (loans mostly stay in their current state;
   `Default` is near-absorbing), not fit from data either. The plumbing —
   feature extraction, belief persistence/continuation across
   assessments, state → loan-status mapping — is real and tested; the
   *predictions* are not calibrated. Train the emission network (Burn
   supports loading a checkpoint via `NeuralEmissionEngine::load`) and
   replace the transition matrix with one fit from historical data before
   trusting the output.

   This was confirmed concretely by a live run against 200 real records
   from the Statlog/German Credit Data set (see "Live run against real
   data" below): with an untrained network, single-step assessments from
   a uniform prior land close to a coin flip between `Current` and
   `Default`, decided more by the hand-specified transition matrix's
   column sums than by the actual (currently weak) emission signal —
   and, in a repeated-reassessment spot check, belief drifted steadily
   *toward* `Current` across several rounds even while the simulated
   payment history was actively getting worse, because the self-transition
   prior compounds with each Bayesian update faster than the untrained
   emission likelihood can push back. This is the expected consequence of
   shipping an untrained network with a hand-specified prior, not a
   separate defect — but it means the system is not yet safe to route
   real servicing decisions through until the emission network is trained
   and the transition matrix is refit.

2. **Related-party traversal rebuilds the whole graph per request**
   (`network_service::related_borrowers` loads every borrower and every
   relationship link, then builds an in-memory `ontology_engine` graph).
   Fine for a lending book at moderate scale (thousands of borrowers);
   replace with an incrementally-maintained graph or a recursive SQL CTE
   if this becomes a hot path at larger scale.

3. **Contagion scoring weights** (`OWN_RISK_WEIGHT = 0.55`,
   `NEIGHBOR_RISK_WEIGHT = 0.30`, `CENTRALITY_WEIGHT = 0.15` in
   `credit_network/src/analysis.rs`) are a reasonable starting blend, not
   fit or validated against outcomes.




## Testing

```bash
# Everything except api/migration (which need a live Postgres):
cargo test --workspace --exclude api --exclude migration

# Vendored crates' own suites (37 tests) + this system's new integration
# tests (9 tests covering hypergraph grouping, contagion propagation,
# related-party BFS, feature normalization, and HMM belief/determinism
# properties):
cargo test -p spectral_hypergraph -p ontology_engine -p neural_hmm \
           -p credit_network -p credit_risk
```


