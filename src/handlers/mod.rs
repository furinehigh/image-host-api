use crate::{config::Config, database::Database, services::{redis::RedisService, metrics::MetricsService}};
use std::sync::Arc;

pub mod auth;
pub mod images;
pub mod admin;
pub mod health;
pub mod docs;
pub mod metrics;

#[derive(Clone)]
pub struct AppState {
    pub database: Database,
    pub redis: RedisService,
    pub config: Config,
    pub metrics: Arc<MetricsService>,
}
