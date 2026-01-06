use aws_config::meta::region::RegionProviderChain;
use aws_credential_types::Credentials;
use aws_types::region::Region;
use aws_sdk_s3::{Client, primitives::ByteStream};
use bytes::Bytes;
use tracing::info;
use async_trait::async_trait;
use crate::{config::Config, storage::{Storage, StorageError}};

// AWS S3 Storage backend
#[derive(Clone)]
pub struct S3Storage{
    client: Client,  // AWS S3 client
    bucket: String,  // S3 bucket name
}

impl S3Storage {
    /// Initialize S3 client and ensure the bucket exists
    pub async fn new(config: &Config) -> Self {
        let region_provider = RegionProviderChain::first_try(Region::new(config.s3_region.clone()))
            .or_default_provider()
            .or_else(Region::new("us-east-1"));

        let mut aws_config_builder = aws_config::from_env().region(region_provider);

        // Custom endpoint (e.g., for MinIO)
        if let Some(endpoint) = &config.s3_endpoint {
            aws_config_builder = aws_config_builder.endpoint_url(endpoint);

            let credentials = Credentials::new(
            config.s3_access_key.clone(),
            config.s3_secret_key.clone(),
            None,
            None,
            "custom"
            );

            aws_config_builder = aws_config_builder.credentials_provider(credentials);
        }

        let aws_config = aws_config_builder.load().await;

        let client = Client::from_conf(
            aws_sdk_s3::config::Builder::from(&aws_config)
                .force_path_style(true)// Required for MinIO
                .build()
        );

        // Ensure bucket exists
        Self::ensure_bucket_exists(&client, &config.s3_bucket).await;

        Self {
            client,
            bucket: config.s3_bucket.clone(),
        }
    }

    /// Ensure the S3 bucket exists, or create it if possible
    async fn ensure_bucket_exists(client: &Client, bucket: &str) {
    // First try to create it directly
    match client.create_bucket().bucket(bucket).send().await {
        Ok(_) => {
            tracing::info!("Bucket {} created successfully", bucket);
        }
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.contains("BucketAlreadyOwnedByYou") || 
               err_msg.contains("BucketAlreadyExists") ||
               err_msg.contains("YourPreviousRequestToCreateTheBucket") {
                tracing::info!("Bucket {} already exists", bucket);
            } else {
                // For MinIO, we might get other errors
                tracing::warn!("Could not create bucket {}: {}", bucket, err_msg);
                // Verify if the bucket exists anyway
                match client.head_bucket().bucket(bucket).send().await {
                    Ok(_) => tracing::info!("Bucket {} exists (verified)", bucket),
                    Err(check_err) => tracing::error!("Bucket {} does not exist and cannot be created: {}", bucket, check_err),
                }
            }
        }
    }
}
}

#[async_trait]
impl Storage for S3Storage {

    /// Uploads content to S3 bucket
    async fn upload(&self, file_path: &str, content: Bytes) -> Result<String, StorageError>{
        let body = ByteStream::from(content);
        
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(file_path)
            .body(body)
            .send()
            .await
            .map_err(|e| StorageError::UploadError(e.to_string()))?;

        Ok(format!("s3://{}", file_path))
    }

    /// Downloads content from S3 bucket
    async fn download(&self, file_path: &str) -> Result<Bytes, StorageError> {
        tracing::info!("S3 GET key = {}", file_path);
        let response = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(file_path)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Wrong key was provided...");
                StorageError::NotFound(e.to_string())
            })?;


        let data = response
            .body
            .collect()
            .await
            .map_err(|e| StorageError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        Ok(data.into_bytes())
    }

    /// Deletes a file from S3 bucket
    async fn delete(&self, file_path: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(file_path)
            .send()
            .await
            .map_err(|e| StorageError::DeleteError(e.to_string()))?;

        info!("File deleted sucessfully from s3: {}", file_path);
        Ok(())
    }

}
