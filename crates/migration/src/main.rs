use migration::{Migrator, MigratorTrait};
use sea_orm_migration::sea_orm::Database;

/// Minimal standalone migration runner (`cargo run -p migration -- up|down|fresh`),
/// used instead of `sea-orm-migration`'s `cli` feature because that feature
/// pulls in `sea-orm-cli`, which does not build on this workspace's pinned
/// rustc 1.75 toolchain (an internal `regex`/`std::error::Error` bound
/// mismatch in `sea-orm-cli` 0.12.15, unrelated to this project's code).
/// `MigratorTrait` alone covers everything this system needs.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/loan_risk".to_string());
    let db = Database::connect(&database_url).await?;

    let command = std::env::args().nth(1).unwrap_or_else(|| "up".to_string());
    match command.as_str() {
        "up" => Migrator::up(&db, None).await?,
        "down" => Migrator::down(&db, Some(1)).await?,
        "fresh" => Migrator::fresh(&db).await?,
        "refresh" => Migrator::refresh(&db).await?,
        "status" => Migrator::status(&db).await?,
        other => {
            eprintln!("unknown command '{other}'; expected one of: up, down, fresh, refresh, status");
            std::process::exit(1);
        }
    }

    Ok(())
}
