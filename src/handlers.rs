use axum::{Json, extract::{Multipart, Path, State}, http::{StatusCode, header}, response::Response};
use bytes::Bytes;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    error::AppError, models::*, state::AppState, storage::Storage, utils::{calculate_sha256, get_file_extension, is_file_mime_type, generate_thumbnail},
};


/// Upload a file using multipart/form-data.
pub async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError>{
    // Temporary holders for multipart fields
    let mut file_data: Option<Bytes> = None;
    let mut original_filename: Option<String> = None;
    let mut mime_type: Option<String> = None;
    let mut file_size: u64 = 0;
    let mut custom_filename: Option<String> = None;

    // Parse multipart fields
    while let Some(field) = multipart.next_field().await.map_err(|e|{
        error!("Error parsing multipart: {}", e);
        AppError::MultipartError(format!("Failed to parse multipart form: {}",e))})? 
        {
        match field.name().unwrap_or("") {
            "file" => {
                original_filename = field.file_name().map(|s| s.to_string());
                mime_type = field.content_type().map(|s| s.to_string());
                // Read file bytes
                let data = field.bytes().await.map_err(|e| {
                    error!("Error reading file bytes: {}", e);
                    AppError::FileProcessingError(format!("Failed to read the file: {}",e))
                })?;
                file_size = data.len() as u64;
                file_data = Some(data);
            }
            "filename" => {
                // Optional custom filename
                if let Ok(name) = field.text().await {
                    if !name.is_empty() {
                        custom_filename = Some(name);
                    }
                }
            }
            _ => {}
        }
    }

    // Ensure file exists
    let file_data = file_data.ok_or_else(|| AppError::BadRequest("No file provided".into()))?;
    let original_filename = original_filename.ok_or_else(|| AppError::BadRequest("No file provided".into()))?;

    // Enforce maximum file size
    if file_size > state.config.max_file_size {
        error!(
        "File size {} exceeds maximum limit of {} bytes",
        file_size,
        state.config.max_file_size
        );

        return Err(AppError::PayloadTooLarge(format!(
            "File size {} exceeds maximum limit of {} bytes",
            file_size, state.config.max_file_size
        )));
    }

    // Validate file extension
    let extension = get_file_extension(&original_filename)
        .ok_or_else(|| AppError::BadRequest("Invalid file extension".into()))?;

    if !state.config.allowed_extensions.contains(&extension) {
        error!("File extension .{} is not allowed",extension);

        return Err(AppError::UnSupportedMediaType(format!(
            "File extension .{} is not allowed",
            extension
        )));
    }

    // Generate unique file ID and filename
    let file_id = Uuid::new_v4();
    let filename = if let Some(custom_name) = custom_filename {
        format!("{}_{}", file_id, custom_name)
    } else {
        format!("{}.{}", file_id, extension)
    };
    let file_path = format!("files/{}", filename);

    // Calculate checksum for deduplication
    let checksum = calculate_sha256(&file_data);

    // Check if file already exists
    let existing_file = sqlx::query_as!(
        File,
        "SELECT * FROM files WHERE checksum = $1 LIMIT 1",
        checksum
    ).fetch_optional(&state.pool)
    .await?;

    if let Some(existing) = existing_file {
        return Ok(Json(UploadResponse { 
            id: existing.id, 
            filename: existing.filename,
            url: format!("/files/{}", existing.id), 
            size: existing.file_size, 
            mime_type: existing.mime_type,
        }));
    }

    // Upload file to storage backend
    let storage_path = state
        .storage.upload(&file_path, file_data.clone())
        .await
        .map_err(|e| {
            error!("Error uploading file: {}",e);
            AppError::InternalServerError("Failed to upload file".into())
        })?; 

    // Generate and upload thumbnail (if supported MIME type)
    let thumbnail_path = if is_file_mime_type(&mime_type.clone().unwrap()) {
        match generate_thumbnail(&file_data, &file_id.to_string()).await {
            Ok(thumb_path) => match tokio::fs::read(&thumb_path).await {
                Ok(thumb_data) => {
                    let thumb_storage_path = format!("thumbnails/{}.jpg", file_id);
                    if state
                        .storage
                        .upload(&thumb_storage_path, Bytes::from(thumb_data))
                        .await
                        .is_ok()
                    {
                        Some(thumb_storage_path)
                    } else {
                        error!("Failed to upload thumbnail");
                        None
                    }
                }
                Err(e) => {
                    error!("Failed to read thumbnail file: {}", e);
                    None
                }
            },
            Err(e) => {
                error!("Failed to generate thumbnail: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Persist file metadata to database
    let file_record = sqlx::query_as!(
        File,
        r#"
        INSERT INTO files (
            id, filename, original_filename, file_path, file_size, mime_type,
            storage_type, checksum, thumbnail_path
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
        RETURNING *
        "#,
        file_id,
        filename,
        original_filename,
        storage_path,
        file_size as i64,
        mime_type.unwrap_or_else(|| "application/octet-stream".into()),
        if state.config.use_s3 { "s3" } else { "local" },
        Some(checksum),
        thumbnail_path
    )
    .fetch_one(&state.pool)
    .await?;

    info!("File uploaded: {} ({} bytes)", file_id, file_size);

    Ok(Json(UploadResponse { 
        id: file_id, 
        filename: file_record.filename, 
        url: format!("/files/{}", file_id), 
        size: file_record.file_size, 
        mime_type: file_record.mime_type,
    }))
}

/// Download a file by its unique ID.
pub async fn download_file(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, AppError> {

    // Fetch file metadata from database
    let file = sqlx::query_as!(
        File,
        "SELECT * FROM files WHERE id = $1",
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;

    // Normalize storage path based on backend type
    // - S3 paths are stored as: s3://files/uuid.ext
    // - Local paths are stored as: uploads/files/uuid.ext
    // Storage backend expects a relative key/path
    let file_path = if file.storage_type == "s3" {
    file.file_path
        .strip_prefix("s3://") 
        .unwrap_or(&file.file_path)
        .to_string()
    } else {
    file.file_path
        .strip_prefix("uploads/")
        .unwrap_or(&file.file_path)
        .to_string()
    };

    // Download file contents from storage
    let content = state.storage.download(&file_path).await.map_err(|e| {
        error!("Error downloading file {}: {}", file_path, e);
        AppError::InternalServerError("Failed to download file".to_string())
    })?;

    // Create HTTP response with binary body 
    let mut response = Response::new(content.into());

    // Set Content-Type header so the browser knows the file type
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_str(&file.mime_type)
            .unwrap_or_else(|_| header::HeaderValue::from_static("application/octet-stream")),
    );

    // Set Content-Disposition header to force download
    // and preserve the original filename
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&format!("attachment; filename=\"{}\"", file.original_filename))
            .unwrap_or_else(|_| header::HeaderValue::from_static("attachment")),
    );

    Ok(response)
}

/// Get metadata for a single file by its ID.
pub async fn get_file(
    State(state): State<AppState>,
    Path(id): Path<Uuid>
) -> Result<Json<FileResponse>, AppError> {

    // Query the database for the file record
    let file = sqlx::query_as!(
        File,
        "SELECT * FROM files WHERE id = $1",
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(||AppError::NotFound("File not found".to_string()))?;

    Ok(Json(FileResponse { 
        id: file.id,
        filename: file.filename, 
        original_filename: file.original_filename, 
        size: file.file_size, 
        mime_type: file.mime_type, 
        uploaded_at: file.uploaded_at, 
        download_url: format!("/files/{}/download", file.id), 
        thumbnail_url: file.thumbnail_path.map(|_| format!("/files/{}/thumbnail", file.id)),
    }))
}

/// Delete a file and its associated resources.
pub async fn delete_file(
    State(state): State<AppState>,
    Path(id): Path<Uuid>
) -> Result<StatusCode, AppError> {

    // Fetch the file record from the database
    let file = sqlx::query_as!(
        File,
        "SELECT * FROM files WHERE id = $1",
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;

    // Resolve the storage-relative file path
    // (remove "s3://" or "uploads/" prefixes)
    let file_path = if file.storage_type == "s3" {
        file.file_path
            .strip_prefix("s3://") 
            .unwrap_or(&file.file_path)
            .to_string()
    } else {
        file.file_path
            .strip_prefix("uploads/")
            .unwrap_or(&file.file_path)
            .to_string()
    };

    // Delete the main file from storage
    state.storage.delete(&file_path).await.map_err(|e| {
        error!("Failed to delete file {}: {:?}", file_path, e);
        AppError::InternalServerError("Failed to delete file from storage".to_string())
    })?;

    // If a thumbnail exists, attempt to delete it as well
    if let Some(thumb_path) = &file.thumbnail_path {
        let thumb_relative_path = if file.storage_type == "s3" {
            thumb_path
                .strip_prefix("s3://")
                .unwrap_or(thumb_path)
                .to_string()
        } else {
            thumb_path
                .strip_prefix("uploads/")
                .unwrap_or(&file.file_path)
                .to_string()
        };

        // Thumbnail deletion failure should not block file deletion
        let _ = state.storage.delete(&thumb_relative_path).await;
    }

    // Remove the file record from the database
    sqlx::query!("DELETE FROM files WHERE id = $1", id)
        .execute(&state.pool)
        .await?;

    info!("File Deleted: {}", id);

    // 204 No Content indicates successful deletion with no response body
    Ok(StatusCode::NO_CONTENT)
}

/// Download and return a file thumbnail.
pub async fn get_thummbnail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>
) -> Result<Response, AppError> {

    // Fetch the file record from the database using the file ID
    let file = sqlx::query_as!(
        File,
        "SELECT * FROM files WHERE id = $1",
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;

    // Ensure the file has an associated thumbnail
    let thumb_path = file.thumbnail_path.ok_or_else(|| {
        AppError::NotFound("Thumbnail not available".to_string())
    })?;

    // Normalize the thumbnail path for the storage backend
    // Removes prefixes like "s3://" or "uploads/"
    let thumb_storage_path = if file.storage_type == "s3" {
        thumb_path
            .strip_prefix("s3://")
            .unwrap_or(&thumb_path)
            .to_string()
    } else {
        thumb_path
            .strip_prefix("uploads/")
            .unwrap_or(&thumb_path)
            .to_string()
    };

    // Download the thumbnail bytes from storage
    let content = state.storage.download(&thumb_storage_path).await.map_err(|_|
        AppError::InternalServerError("Failed to download thumbnail".to_string())
    )?;

    // Create an HTTP response with the binary thumbnail data
    let mut response = Response::new(content.into());

    // Explicitly set the content type to JPEG
    // This allows browsers and clients to correctly render the image
    response.headers_mut().insert(
        header::CONTENT_TYPE, 
        header::HeaderValue::from_static("image/jpeg")
    );

    Ok(response)
}

/// List recently uploaded files.
pub async fn list_files(
    State(state): State<AppState>
) -> Result<Json<Vec<FileResponse>>, AppError> {

    // Fetch the most recent 100 file records from the database
    let files = sqlx::query_as!(
        File,
        "SELECT * FROM files ORDER BY uploaded_at DESC LIMIT 100",
    )
    .fetch_all(&state.pool)
    .await?;

    // Transform database File models into FileResponse objects
    // suitable for API output
    let response = files.into_iter().map(|file| {
        FileResponse {
            id: file.id,
            filename: file.filename,
            original_filename: file.original_filename,
            size: file.file_size,
            mime_type: file.mime_type,
            uploaded_at: file.uploaded_at,
            download_url: format!("/files/{}/download", file.id),
            thumbnail_url: file.thumbnail_path.map(|_| format!("files/{}/thumbnail", file.id))
        }
    }).collect();

    // Return the list as a JSON array
    Ok(Json(response))
}