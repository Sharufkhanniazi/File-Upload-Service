mod models;
mod utils;
mod database;
mod config;
mod state;
mod storage;
mod handlers;
mod error;

use axum::{routing::{post, get, delete}, Router};
use std::net::SocketAddr;
use tracing_subscriber;
use tracing::info;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    handlers::{upload_file, download_file, delete_file, get_thummbnail, get_file, list_files},
    state::AppState,
    config::Config,
    database::init_db,
    storage::init_storage,
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    let config = Config::from_env()
        .expect("Failed to load configuration");

    let pool = init_db(&config.database_url)
        .await
        .expect("Failed to connect to db");

    let storage = init_storage(&config).await;

    let app_state = AppState {
        pool,
        storage,
        config
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/upload", post(upload_file))
        .route("/files/{id}/download", get(download_file))
        .route("/files/{id}/thumbnail", get(get_thummbnail))
        .route("/files/{id}", get(get_file))
        .route("/files", get(list_files))
        .route("/files/{id}", delete(delete_file))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);
    
    let addr = SocketAddr::from(([0,0,0,0], 3000));
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
