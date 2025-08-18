// src/handlers/profile_handlers.rs - NEW FILE
use actix_web::{get, put, web, HttpResponse, Responder};
use serde::Serialize;
use crate::services::auth_services::AuthService;
use crate::middleware::auth_extractor::AuthenticatedUser;
use crate::dtos::personal::{PersonalDataOut, CreatePersonalDTO};
use chrono::NaiveDate;

#[derive(Serialize)]
struct ApiResponse<T: serde::Serialize> {
    status: String,
    message: String,
    data: Option<T>,
}

#[derive(Serialize)]
struct UserProfileResponse {
    pub user_id: String,
    pub email: String,
    pub username: Option<String>,
    pub profile: Option<PersonalDataOut>,
}

/// GET /api/profile
/// Get current user's profile data
#[get("/api/profile")]
pub async fn get_user_profile(
    auth_user: AuthenticatedUser,
    svc: web::Data<AuthService>,
) -> impl Responder {
    println!("=== GET PROFILE DEBUG ===");
    println!("User ID from auth: {}", auth_user.user_id);

    // inside get_user_profile
    match svc.get_user_profile(auth_user.user_id).await {
        Ok(profile_opt) => {
            // get basic user info (optional)
            let user_info = match get_user_basic_info(&svc, auth_user.user_id).await {
                Ok(info) => info,
                Err(_) => UserBasicInfo { email: "".to_string(), username: None },
            };

            // If profile exists, return PersonalDataOut directly in data
            if let Some(profile) = profile_opt {
                // combine some basic info into response message if you want
                HttpResponse::Ok().json(ApiResponse {
                    status: "success".to_string(),
                    message: "Profile retrieved successfully".to_string(),
                    data: Some(profile),
                })
            } else {
                // return empty data with success (frontend will fallback to local)
                HttpResponse::Ok().json(ApiResponse::<PersonalDataOut> {
                    status: "success".to_string(),
                    message: "No profile found".to_string(),
                    data: None,
                })
            }
        }
        Err(e) => {
            println!("Failed to get user profile: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                status: "error".to_string(),
                message: "Failed to retrieve profile".to_string(),
                data: None,
            })
        }
    }

}

/// PUT /api/profile
/// Update user's profile data
#[put("/api/profile")]
pub async fn update_user_profile(
    auth_user: AuthenticatedUser,
    svc: web::Data<AuthService>,
    body: web::Json<CreatePersonalDTO>,
) -> impl Responder {
    println!("=== UPDATE PROFILE DEBUG ===");
    println!("User ID: {}", auth_user.user_id);
    println!("Update data: {:?}", body);

    // Convert to ISO format for storage
    // allow empty date -> treat as empty string (clear)
    let iso_date = if body.date_of_birth.trim().is_empty() {
        "".to_string()
    } else {
        // try first YYYY-MM-DD then DD/MM/YYYY
        match NaiveDate::parse_from_str(&body.date_of_birth, "%Y-%m-%d")
            .or_else(|_| NaiveDate::parse_from_str(&body.date_of_birth, "%d/%m/%Y"))
        {
            Ok(d) => d.format("%Y-%m-%d").to_string(),
            Err(_) => {
                return HttpResponse::BadRequest().json(ApiResponse::<()> {
                    status: "error".to_string(),
                    message: "Invalid date format. Use YYYY-MM-DD or DD/MM/YYYY".to_string(),
                    data: None,
                })
            }
        }
};

    let profile_dto = CreatePersonalDTO {
        date_of_birth: iso_date,
        primary_skill: body.primary_skill.clone(),
        skill_to_learn: body.skill_to_learn.clone(),
        bio: body.bio.clone(),
    };

    match update_profile_data(&svc, auth_user.user_id, profile_dto).await {
        Ok(updated_profile) => {
            HttpResponse::Ok().json(ApiResponse {
                status: "success".to_string(),
                message: "Profile updated successfully".to_string(),
                data: Some(updated_profile),
            })
        }
        Err(e) => {
            println!("Failed to update profile: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                status: "error".to_string(),
                message: "Failed to update profile".to_string(),
                data: None,
            })
        }
    }
}

// Helper structures for user basic info
#[derive(serde::Deserialize)]
struct UserBasicInfo {
    email: String,
    username: Option<String>,
}

// Helper function to get user basic info
async fn get_user_basic_info(
    svc: &AuthService,
    user_id: uuid::Uuid,
) -> Result<UserBasicInfo, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/rest/v1/profiles", svc.supabase_url);
    
    let response = svc.client
        .get(&url)
        .header("apikey", &svc.supabase_anon_key)
        .header("Authorization", format!("Bearer {}", &svc.supabase_service_role_key))
        .query(&[("id", format!("eq.{}", user_id)), ("select", "email,username".to_string())])
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to get user info: {}", response.status()).into());
    }

    let users: Vec<serde_json::Value> = response.json().await?;
    
    if let Some(user_data) = users.first() {
        Ok(UserBasicInfo {
            email: user_data["email"].as_str().unwrap_or("").to_string(),
            username: user_data["username"].as_str().map(|s| s.to_string()),
        })
    } else {
        Err("User not found".into())
    }
}

// Helper function to update profile data
async fn update_profile_data(
    svc: &AuthService,
    user_id: uuid::Uuid,
    profile_dto: CreatePersonalDTO,
) -> Result<PersonalDataOut, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/rest/v1/profiles", svc.supabase_url);
    
    let update_data = serde_json::json!({
        "date_of_birth": profile_dto.date_of_birth,
        "primary_skill": profile_dto.primary_skill,
        "skill_to_learn": profile_dto.skill_to_learn,
        "bio": profile_dto.bio,
    });

    let response = svc.client
        .patch(&url)
        .header("apikey", &svc.supabase_service_role_key)
        .header("Authorization", format!("Bearer {}", &svc.supabase_service_role_key))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=representation")
        .query(&[("id", format!("eq.{}", user_id))])
        .json(&update_data)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to update profile: {} - {}", response.status(), error_text).into());
    }

    let updated_profiles: Vec<serde_json::Value> = response.json().await?;
    
    if let Some(profile_data) = updated_profiles.first() {
        Ok(PersonalDataOut {
            id: serde_json::from_value(profile_data["id"].clone())?,
            user_id: serde_json::from_value(profile_data["id"].clone())?, // Use id as user_id for profiles table
            date_of_birth: profile_data["date_of_birth"].as_str().unwrap_or("").to_string(),
            primary_skill: profile_data["primary_skill"].as_str().unwrap_or("").to_string(),
            skill_to_learn: profile_data["skill_to_learn"].as_str().unwrap_or("").to_string(),
            bio: profile_data["bio"].as_str().unwrap_or("").to_string(),
            profile_picture_url: profile_data["profile_picture_url"].as_str().map(|s| s.to_string()),
        })
    } else {
        Err("No profile data returned from update".into())
    }
}