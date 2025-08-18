// src/dtos/profile_picture_dtos.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct UploadProfilePictureRequest {
    pub image_data: String, // base64 encoded image
    pub file_name: String,
    pub content_type: String, // "image/jpeg", "image/png", etc.
}

#[derive(Serialize)]
pub struct ProfilePictureResponse {
    pub profile_picture_url: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct SkipProfilePictureResponse {
    pub message: String,
    pub next_step: String,
}