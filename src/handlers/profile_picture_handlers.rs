// src/handlers/profile_picture_handlers.rs - FIXED VERSION
use actix_web::{post, web, HttpResponse, Responder};
use base64::{Engine as _, engine::general_purpose};
use uuid::Uuid;
use serde::Serialize;
use crate::middleware::auth_extractor::AuthenticatedUser;
use crate::dtos::profile_picture_dtos::{UploadProfilePictureRequest, ProfilePictureResponse, SkipProfilePictureResponse};
use crate::services::auth_services::AuthService;
use std::path::Path;

#[derive(Serialize)]
struct ApiResponse<T: serde::Serialize> {
    status: String,
    message: String,
    data: Option<T>,
}

/// POST /api/profile-picture/upload
/// Upload profile picture (authenticated endpoint)
#[post("/api/profile-picture/upload")]
pub async fn upload_profile_picture(
    auth_user: AuthenticatedUser,
    svc: web::Data<AuthService>,
    body: web::Json<UploadProfilePictureRequest>,
) -> impl Responder {
    let user_id = auth_user.user_id;
    
    println!("=== UPLOAD PROFILE PICTURE DEBUG ===");
    println!("User ID: {}", user_id);
    println!("Content Type: {}", body.content_type);
    println!("File Name: {}", body.file_name);
    println!("Image data length: {}", body.image_data.len());

    // Validate content type
    let allowed_types = ["image/jpeg", "image/jpg", "image/png", "image/gif", "image/webp"];
    if !allowed_types.contains(&body.content_type.as_str()) {
        println!("Invalid content type: {}", body.content_type);
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            status: "error".to_string(),
            message: "Invalid file type. Only JPEG, PNG, GIF, and WEBP are allowed.".to_string(),
            data: None,
        });
    }

    // Remove data URL prefix if present (data:image/jpeg;base64,)
    let base64_data = if body.image_data.contains(',') {
        let split_data = body.image_data.split(',').nth(1).unwrap_or(&body.image_data);
        println!("Removed data URL prefix");
        split_data
    } else {
        &body.image_data
    };

    // Decode base64
    let image_bytes = match general_purpose::STANDARD.decode(base64_data) {
        Ok(bytes) => {
            println!("Successfully decoded base64, {} bytes", bytes.len());
            bytes
        },
        Err(e) => {
            println!("Failed to decode base64: {}", e);
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                status: "error".to_string(),
                message: "Invalid base64 image data".to_string(),
                data: None,
            });
        }
    };

    // Generate unique filename
    let extension = match body.content_type.as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "jpg", // fallback
    };
    
    let filename = format!("{}_profile.{}", user_id, extension);
    println!("Generated filename: {}", filename);
    
    // For development, save to local storage
    let upload_dir = "uploads/profile_pictures";
    
    // Create directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(upload_dir) {
        println!("Failed to create upload directory: {}", e);
        return HttpResponse::InternalServerError().json(ApiResponse::<()> {
            status: "error".to_string(),
            message: "Failed to prepare file storage".to_string(),
            data: None,
        });
    }

    let file_path = format!("{}/{}", upload_dir, filename);
    println!("Saving to: {}", file_path);
    
    // Save file
    if let Err(e) = std::fs::write(&file_path, &image_bytes) {
        println!("Failed to save profile picture: {}", e);
        return HttpResponse::InternalServerError().json(ApiResponse::<()> {
            status: "error".to_string(),
            message: "Failed to save profile picture".to_string(),
            data: None,
        });
    }

    println!("File saved successfully!");

    // Generate public URL (adjust this based on your setup)
    let public_url = format!("/api/uploads/profile_pictures/{}", filename);
    println!("Public URL: {}", public_url);

    // Update user profile with picture URL
    println!("Updating database...");
    match svc.update_profile_picture(user_id, Some(public_url.clone())).await {
        Ok(_) => {
            println!("Database updated successfully!");
            let response = ProfilePictureResponse {
                profile_picture_url: public_url,
                message: "Profile picture uploaded successfully!".to_string(),
            };

            HttpResponse::Ok().json(ApiResponse {
                status: "success".to_string(),
                message: "Profile picture uploaded".to_string(),
                data: Some(response),
            })
        }
        Err(e) => {
            println!("Failed to update profile picture in database: {}", e);
            
            // Clean up uploaded file if database update fails
            let _ = std::fs::remove_file(&file_path);
            
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                status: "error".to_string(),
                message: "Failed to save profile picture information".to_string(),
                data: None,
            })
        }
    }
}

/// POST /api/profile-picture/skip
/// Skip profile picture upload (authenticated endpoint)
#[post("/api/profile-picture/skip")]
pub async fn skip_profile_picture(
    _auth_user: AuthenticatedUser,
) -> impl Responder {
    println!("=== SKIP PROFILE PICTURE ===");
    let response = SkipProfilePictureResponse {
        message: "Profile picture skipped. You can add one later from your profile settings.".to_string(),
        next_step: "dashboard".to_string(),
    };

    HttpResponse::Ok().json(ApiResponse {
        status: "success".to_string(),
        message: "Profile setup completed".to_string(),
        data: Some(response),
    })
}

/// GET /api/uploads/profile_pictures/{filename}
/// Serve uploaded profile pictures (public endpoint for development)
#[actix_web::get("/api/uploads/profile_pictures/{filename}")]
pub async fn serve_profile_picture(path: web::Path<String>) -> impl Responder {
    let filename = path.into_inner();
    
    // Sanitize filename to prevent directory traversal
    let safe_filename = Path::new(&filename)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("invalid");
    
    let file_path = format!("uploads/profile_pictures/{}", safe_filename);
    
    match std::fs::read(&file_path) {
        Ok(data) => {
            // FIXED: Add WEBP content type support
            let content_type = match Path::new(&safe_filename)
                .extension()
                .and_then(|ext| ext.to_str()) {
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("png") => "image/png",
                Some("gif") => "image/gif",
                Some("webp") => "image/webp", // ADDED WEBP
                _ => "application/octet-stream",
            };

            HttpResponse::Ok()
                .content_type(content_type)
                .body(data)
        }
        Err(_) => {
            HttpResponse::NotFound().json(serde_json::json!({
                "status": "error",
                "message": "Profile picture not found"
            }))
        }
    }
}