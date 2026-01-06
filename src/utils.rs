use std::path::Path;
use sha2::{Digest, Sha256};

/// Extracts the file extension from a filename and converts it to lowercase.
pub fn get_file_extension(filename: &str) -> Option<String> {
    Path::new(filename) // treats string as filesystem path.
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
}

/// Calculates SHA-256 checksum of the given data slice.
pub fn calculate_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
    // {:x} means format the value as lowercase hexadecimal string
}

/// Checks if a MIME type represents an image.
pub fn is_file_mime_type(mime_type: &str) -> bool {
    mime_type.starts_with("image/")
}

/// Generates a thumbnail image from the given file data asynchronously.
pub async fn generate_thumbnail(
    data: &[u8],
    base_name: &str
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let data = data.to_vec();
    let base = base_name.to_string();

    tokio::task::spawn_blocking(move || { // spawn_blocking used when cpu heavy work so other task don't stop processing
        // Load image from memory bytes
        let img = image::load_from_memory(&data)?;

        // doing this directly without spawn_blocking in async code would block the executor.

        // Resize image to a thumbnail (max width/height = 200px)
        let thumnail= img.thumbnail(200, 200);

        // Get system temporary directory (OS-specific)
        let temp_dir = std::env::temp_dir(); // it is path for temp files every os has one.

        // Construct temporary output path for thumbnail
        let output_path = temp_dir.join(format!("{}_thumb.jpg", base));

        // Save thumbnail as JPEG
        thumnail.save_with_format(&output_path, image::ImageFormat::Jpeg)?;

        // Convert PathBuf to String safely
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
            output_path.to_string_lossy().into_owned()
        ) // to_string_lossy converts PathBuf to Cow<str>.
    }).await?
}