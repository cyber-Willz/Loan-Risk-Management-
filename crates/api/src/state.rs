use sea_orm::DatabaseConnection;

use crate::risk_actor::RiskActorHandle;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub risk_actor: RiskActorHandle,
}
