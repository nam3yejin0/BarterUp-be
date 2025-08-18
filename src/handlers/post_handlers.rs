// src/handlers/post_handlers.rs - Updated with proper profile support for logged-in users

use actix_web::{post, web, get, HttpResponse};
use crate::dtos::post_dtos::CreatePostDTO;
use crate::repositories::post_repository::{PostRepository, PostWithProfile};
use crate::middleware::auth_extractor::AuthenticatedUser;
use crate::AppState;

#[derive(serde::Serialize)]
struct ApiResponse<T: serde::Serialize> {
    status: String,
    message: String,
    data: Option<T>,
}

// Add Debug derive to fix the compilation error
#[derive(Debug, serde::Serialize)]
pub struct EnhancedPostOut {
    pub id: String,
    pub user_id: String,
    pub content: Option<String>,
    pub image_url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    // Enhanced fields for frontend
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub author_role: String,
    pub author_primary_skill: Option<String>,
    pub is_own_post: bool,
}

#[post("/posts")]
pub async fn create_post(
    app_state: web::Data<AppState>,
    user: AuthenticatedUser,
    body: web::Json<CreatePostDTO>,
) -> HttpResponse {
    println!("=== CREATE POST DEBUG ===");
    println!("User ID: {}", user.user_id);
    println!("Content: {}", body.content);
    println!("Image URL: {:?}", body.image_url);

    match PostRepository::create_post(
        &app_state.supabase_url,
        &app_state.supabase_key,
        &app_state.http_client,
        user.user_id,
        body.into_inner(),
    ).await {
        Ok(post) => {
            println!("Post created successfully: {:?}", post);
            HttpResponse::Ok().json(ApiResponse {
                status: "success".to_string(),
                message: "Post created successfully".to_string(),
                data: Some(post),
            })
        }
        Err(e) => {
            println!("Failed to create post: {:?}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                status: "error".to_string(),
                message: format!("Failed to create post: {}", e),
                data: None,
            })
        }
    }
}

#[get("/posts")]
pub async fn list_posts(
    app_state: web::Data<AppState>,
    user: Option<AuthenticatedUser>,
) -> HttpResponse {
    println!("=== LIST POSTS WITH PROFILES DEBUG ===");
    
    let current_user_id = user.as_ref().map(|u| u.user_id.to_string());
    println!("Current user ID: {:?}", current_user_id);
    
    match PostRepository::list_posts_with_profiles(
        &app_state.supabase_url,
        &app_state.supabase_key,
        &app_state.http_client,
        50
    ).await {
        Ok(posts) => {
            println!("Posts with profiles retrieved: {} items", posts.len());
            
            // Transform posts to enhanced format
            let enhanced_posts: Vec<EnhancedPostOut> = posts
                .into_iter()
                .map(|post| {
                    println!("Processing post: ID={}, UserID={}, Profile={:?}", 
                            post.id, post.user_id, post.profiles);
                    transform_post_with_profile(post, current_user_id.as_deref())
                })
                .collect();
            
            println!("Enhanced posts: {:?}", enhanced_posts);
            
            HttpResponse::Ok().json(ApiResponse {
                status: "success".to_string(),
                message: "Posts retrieved successfully".to_string(),
                data: Some(enhanced_posts),
            })
        }
        Err(e) => {
            println!("Failed to list posts with profiles: {:?}", e);
            
            // Fallback to basic posts if profile join fails
            println!("Falling back to basic posts...");
            match PostRepository::list_posts(
                &app_state.supabase_url,
                &app_state.supabase_key,
                &app_state.http_client,
                50
            ).await {
                Ok(basic_posts) => {
                    let enhanced_posts: Vec<EnhancedPostOut> = basic_posts
                        .into_iter()
                        .map(|post| transform_basic_post(post, current_user_id.as_deref()))
                        .collect();
                    
                    HttpResponse::Ok().json(ApiResponse {
                        status: "success".to_string(),
                        message: "Posts retrieved successfully (basic mode)".to_string(),
                        data: Some(enhanced_posts),
                    })
                }
                Err(e2) => {
                    println!("Failed to retrieve basic posts: {:?}", e2);
                    HttpResponse::InternalServerError().json(ApiResponse::<()> {
                        status: "error".to_string(),
                        message: "Failed to retrieve posts".to_string(),
                        data: None,
                    })
                }
            }
        }
    }
}

/// Transform PostWithProfile to EnhancedPostOut
fn transform_post_with_profile(post: PostWithProfile, current_user_id: Option<&str>) -> EnhancedPostOut {
    let profile = post.profiles.as_ref();
    let is_own_post = current_user_id == Some(&post.user_id);
    
    println!("Transform debug - Post user: {}, Current user: {:?}, Is own: {}", 
             post.user_id, current_user_id, is_own_post);
    
    // Use profile data if available, otherwise fallback to defaults
    let author_name = profile
        .and_then(|p| p.full_name.clone())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| {
            if is_own_post {
                "You".to_string()  // Show "You" for current user's posts
            } else {
                "Anonymous User".to_string()
            }
        });
    
    let author_role = profile
        .and_then(|p| p.role.clone())
        .filter(|role| !role.trim().is_empty())
        .or_else(|| profile.and_then(|p| p.primary_skill.clone()))
        .filter(|skill| !skill.trim().is_empty())
        .unwrap_or_else(|| "User".to_string());
    
    let author_avatar = profile
        .and_then(|p| p.profile_picture_url.clone())
        .filter(|url| !url.trim().is_empty());
    
    let author_primary_skill = profile
        .and_then(|p| p.primary_skill.clone())
        .filter(|skill| !skill.trim().is_empty());
    
    EnhancedPostOut {
        id: post.id,
        user_id: post.user_id.clone(),
        content: post.content,
        image_url: post.image_url,
        created_at: post.created_at,
        updated_at: post.updated_at,
        author_name,
        author_avatar,
        author_role,
        author_primary_skill,
        is_own_post,
    }
}

/// Transform basic PostOut to EnhancedPostOut (fallback)
fn transform_basic_post(post: crate::dtos::post_dtos::PostOut, current_user_id: Option<&str>) -> EnhancedPostOut {
    let post_user_id = post.user_id.as_ref().map(|s| s.as_str()).unwrap_or("");
    let is_own_post = current_user_id == Some(post_user_id) && !post_user_id.is_empty();
    
    EnhancedPostOut {
        id: post.id,
        user_id: post.user_id.unwrap_or_default(),
        content: post.content,
        image_url: post.image_url,
        created_at: post.created_at,
        updated_at: post.updated_at,
        author_name: if is_own_post { "You".to_string() } else { "Member".to_string() },
        author_avatar: None,
        author_role: "User".to_string(),
        author_primary_skill: None,
        is_own_post,
    }
}