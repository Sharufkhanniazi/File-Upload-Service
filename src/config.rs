use std::env;

use dotenvy::dotenv;
use validator::Validate;

#[derive(Debug, Clone, Validate)]
pub struct Config {
    pub database_url: String,
    pub s3_endpoint: Option<String>,
    pub s3_region: String,
    pub s3_bucket: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    #[validate(range(min = 1, max = 104857600))] // Max 100MB
    pub max_file_size: u64,
    pub allowed_extensions: Vec<String>,
    pub use_s3: bool,
}

impl Config {
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self, env::VarError> {
        // Load environment variables from `.env` file (if it exists)
        dotenv().ok();

        let allowed_extensions = env::var("ALLOWED_EXTENSIONS")
            .unwrap_or_else(|_| "jpg,jpeg,png,gif,pdf,doc,docx,txt".to_string())
            .split(',')
            .map(|s| s.to_lowercase())
            .collect();

        let config = Config {
            database_url: env::var("DATABASE_URL")?,
            s3_endpoint: env::var("S3_ENDPOINT").ok(),
            s3_region: env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            s3_bucket: env::var("S3_BUCKET").unwrap_or_else(|_| "file-service".to_string()),
            s3_access_key: env::var("S3_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".to_string()),
            s3_secret_key: env::var("S3_SECRET_KEY").unwrap_or_else(|_| "minioadmin".to_string()),
            max_file_size: env::var("MAX_FILE_SIZE")
                .unwrap_or_else(|_| "10485760".to_string())
                .parse()
                .unwrap_or(10_485_760),
            allowed_extensions,
            use_s3: env::var("USE_S3")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        };
        
        // Validate configuration values (e.g. file size range)
        config.validate().expect("Invalid Configuration");
        Ok(config)

    }
}