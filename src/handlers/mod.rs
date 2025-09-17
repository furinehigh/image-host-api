pub mod health;
pub mod upload;
pub mod images;
pub mod user;
pub mod metrics;
pub mod docs;
pub mod auth; // Add auth module

use crate::{config::Config, database::Database, services::redis::RedisService};

#[derive(Clone)]
pub struct AppState {
    pub database: Database,
    pub redis: RedisService,
    pub config: Config,
}
