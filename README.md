# Rust File Service

A simple file upload/download service built with **Rust**, **Axum**, **PostgreSQL**, and optional **S3/MinIO** storage.  
Supports file uploads with MIME type validation, SHA-256 checksum deduplication, and automatic thumbnail generation for images.

---

## Features

- Upload files via `multipart/form-data`.
- Deduplicate files using SHA-256 checksums.
- Generate and serve thumbnails for image files.
- Store files locally or in S3/MinIO.
- RESTful endpoints for:
  - Uploading files
  - Downloading files
  - Retrieving file metadata
  - Listing recent files
  - Deleting files
- Configurable via environment variables.

---

## Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/upload` | POST | Upload a file (supports custom filename) |
| `/files/{id}/download` | GET | Download file by ID |
| `/files/{id}/thumbnail` | GET | Download thumbnail (if exists) |
| `/files/{id}` | GET | Get file metadata |
| `/files` | GET | List recent files |
| `/files/{id}` | DELETE | Delete a file by ID |

---

## Example Usage

### Upload a file
```bash
curl -X POST http://localhost:3000/upload \
-F "file=@/path/to/image.png" \
-F "filename=custom_name.png"

Request =>
     curl -X POST http://localhost:3000/upload \
     -F "file=@/home/sharuf-khan/Pictures/Screenshots/days.png" \
     -F "filename=daysGone.png"

Response => 
    {
    "id":"1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5",
    "filename":"1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5_daysGone.png",
    "url":"/files/1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5",
    "size":37634,
    "mime_type":"image/png"
    }

### Download a file

Request => 
    curl -L -o daysGone.png http://localhost:3000/files/1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5/download 

Response => 
    % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current
                                    Dload  Upload   Total   Spent    Left  Speed
    100 37634  100 37634    0     0  1776k      0 --:--:-- --:--:-- --:--:-- 1837k  


### Download a thumbnail

Request => 
    curl -L -o daysGone.png http://localhost:3000/files/1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5/thumbnail
    
Response =>
    % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current
                                       Dload  Upload   Total   Spent    Left  Speed
    100  3433  100  3433    0     0   244k      0 --:--:-- --:--:-- --:--:--  257k 


### Get file by Id

Request =>
curl http://localhost:3000/files/1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5 -o output.png

Terminal Response =>   % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current
                                                       Dload  Upload   Total   Spent    Left  Speed
                    100   362  100   362    0     0   9197      0 --:--:-- --:--:-- --:--:--  9282

Browser Response =>
  {
  "id": "1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5",
  "filename": "1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5_daysGone.png",
  "original_filename": "days.png",
  "size": 37634,
  "mime_type": "image/png",
  "uploaded_at": "2026-01-05T10:31:22.500536Z",
  "download_url": "/files/1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5/download",
  "thumbnail_url": "/files/1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5/thumbnail"
  }


### Get all files

Request =>
    curl http://localhost:3000/files
            
Response =>       
    [{
    "id":"1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5",      
    "filename":"1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5_daysGone.png",
    "original_filename":"days.png",
    "size":37634,
    "mime_type":"image/png",
    "uploaded_at":"2026-01-05T10:31:22.500536Z",
    "download_url":"/files/1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5/download",
    "thumbnail_url":"files/1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5/thumbnail"
    }]




### Delete file by Id

Request => 
    curl -X DELETE http://localhost:3000/files/1d0cc4fe-d48a-4caf-b8e2-29a0d8de29a5e2-29a0d8de29a5
        