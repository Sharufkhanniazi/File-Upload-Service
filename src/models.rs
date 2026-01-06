use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use chrono::{DateTime, Utc};
use uuid::Uuid;


#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct File {
    pub id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub storage_type: String,
    pub checksum: Option<String>,
    pub thumbnail_path: Option<String>,
    pub uploaded_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub size: i64,
    pub mime_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileResponse {
    pub id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub size: i64,
    pub mime_type: String,
    pub uploaded_at: Option<DateTime<Utc>>,
    pub download_url: String,
    pub thumbnail_url: Option<String>,
}
