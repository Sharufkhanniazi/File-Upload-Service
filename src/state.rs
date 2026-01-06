use sqlx::PgPool;
use crate::storage::StorageBackend;
use crate::config::Config;

/// Central application state shared across all Axum handlers.
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL connection pool.
    pub pool: PgPool,

    /// Abstracted storage backend (local filesystem or S3).
    pub storage: StorageBackend,
    
    /// Application configuration loaded from environment variables or `.env`.
    pub config: Config,
}