use std::path::Path;
use bytes::Bytes;
use super::{Storage, StorageError};
use async_trait::async_trait;
use tokio::{fs, io::AsyncWriteExt};

// Local filesystem storage
#[derive(Clone)]
pub struct LocalStorage{
    base_path: String, // Base directory where files will be stored
}

impl LocalStorage {
    pub async fn new(base_path: &str) -> Self {
        /// Creates a new LocalStorage instance and ensures necessary directories exist
        fs::create_dir_all(base_path).await.expect("Failed to create uploads directory");
        fs::create_dir_all(format!("{}/files",base_path)).await.expect("Failed to create files directory");
        fs::create_dir_all(format!("{}/thumbnails",base_path)).await.expect("Failed to create thumbnails directory");
        Self {
            base_path: base_path.to_string(),
        }
    }
    /// Returns the full path of a file relative to the base directory
    fn get_full_path(&self, file_path: &str) -> String {
        format!("{}/{}", self.base_path, file_path)
    }
}

#[async_trait]
impl Storage for LocalStorage {

    /// Uploads content to a file on the local filesystem
    async fn upload(&self, file_path: &str, content: Bytes)
    -> Result<String, StorageError> {
        
        let full_path = self.get_full_path(file_path);

        // Ensure parent directories exist
        if let Some(parent) = Path::new(&full_path).parent() {
            fs::create_dir_all(parent).await?;
        }

        // Create the file and write content
        let mut file = fs::File::create(&full_path).await?;
        file.write_all(&content).await?;

        tracing::info!("Saved file at {:?}", full_path);

        Ok(full_path)
    }

    /// Downloads a file from local filesystem
    async fn download(&self, file_path: &str) -> Result<Bytes, StorageError> {
        let full_path = self.get_full_path(file_path);

        if !Path::new(&full_path).exists() {
            return Err(StorageError::NotFound(file_path.to_string()));
        }

        let content = fs::read(&full_path).await
            .map_err(|e| StorageError::IoError(e))?;

        Ok(Bytes::from(content))
    }

    /// Deletes a file from local filesystem
    async fn delete(&self, file_path: &str) -> Result<(), StorageError> {
        let full_path = self.get_full_path(file_path);

        if Path::new(&full_path).exists() {
            fs::remove_file(&full_path)
                .await
                .map_err(|e| StorageError::IoError(e))?;
        }
        Ok(())
    }
}