use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::info;

/// Initialize and return a PostgreSQL connection pool.
pub async fn init_db(database_url: &str) -> Result<PgPool, sqlx::Error> {
    info!("Connecting to database...");

    // Create a new PostgreSQL connection pool with a maximum of 5 connections
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    info!("Database connection established");
    Ok(pool)
}