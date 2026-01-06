// Submodules for local file system storage and S3 storage
mod local;
mod s3;

use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;
use tracing::info;

use crate::{
    storage::{local::LocalStorage, s3::S3Storage},
    config::Config,
};

// Storage error types
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("File not found: {0}")]
    NotFound(String), // Returned when a file cannot be found

    #[error("Io Error: {0}")]
    IoError(#[from] std::io::Error), // Wraps standard I/O errors

    #[error("Upload Error: {0}")]
    UploadError(String), // Errors during upload to storage

    #[error("Delete Error: {0}")]
    DeleteError(String) // Errors during deletion from storage
}

// Async Storage trait
#[async_trait]
pub trait Storage: Send + Sync {
    /// Upload a file to the storage backend.
    /// Returns the full path or key of the uploaded file.
    async fn upload(&self, file_path: &str, content: Bytes) -> Result<String, StorageError>;

    /// Download a file from the storage backend.
    /// Returns the file content as `Bytes`.
    async fn download(&self, file_path: &str) -> Result<Bytes, StorageError>;

    /// Delete a file from the storage backend.
    async fn delete(&self, file_path: &str) -> Result<(), StorageError>;
}

// Enum to represent storage backends
#[derive(Clone)]
pub enum StorageBackend {
    Local(LocalStorage),  // Local filesystem storage
    S3(S3Storage),        // AWS S3 or MinIO storage
}

// Implement Storage trait for StorageBackend enum
// Delegates calls to the chosen backend
#[async_trait]
impl Storage for StorageBackend {
    async fn upload(&self, file_path: &str, content: Bytes) -> Result<String, StorageError>{
        match self {
            StorageBackend::Local(s) => s.upload(file_path, content).await,
            StorageBackend::S3(s) => s.upload(file_path, content).await,
        }
    }

    async fn download(&self, file_path: &str) -> Result<Bytes, StorageError> {
        match self {
            StorageBackend::Local(s) => s.download(file_path).await,
            StorageBackend::S3(s) => s.download(file_path).await,
        }
    }

    async fn delete(&self, file_path: &str) -> Result<(), StorageError> {
        match self {
            StorageBackend::Local(s) => s.delete(file_path).await,
            StorageBackend::S3(s) => s.delete(file_path).await,
        }
    }
}

// Initialize the storage backend based on config
pub async fn init_storage(config: &Config) -> StorageBackend {
    if config.use_s3 {
        info!("Initializing S3 storage");
        StorageBackend::S3(S3Storage::new(config).await)
    } else {
        info!("Initializing Local storage");
        StorageBackend::Local(LocalStorage::new("uploads").await)
    }
}