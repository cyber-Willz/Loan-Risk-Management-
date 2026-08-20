use std::time::Duration;

use api::risk_actor::RiskActorHandle;
use api::routes;
use api::state::AppState;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/loan_risk".to_string());
    let mut opts = ConnectOptions::new(database_url);
    opts.max_connections(20)
        .min_connections(2)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8));

    let db = Database::connect(opts).await?;

    if std::env::var("SKIP_MIGRATIONS").is_err() {
        Migrator::up(&db, None).await?;
        tracing::info!("migrations applied");
    }

    let risk_actor = RiskActorHandle::spawn()?;
    let state = AppState { db, risk_actor };

    let app = routes::router()
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "loan risk management API listening");
    axum::serve(listener, app).await?;

    Ok(())
}
