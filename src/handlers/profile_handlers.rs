// src/handlers/profile_handlers.rs
use actix_web::{get, put, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use crate::services::auth_services::AuthService;
use crate::middleware::auth_extractor::AuthenticatedUser;
use crate::dtos::personal::{PersonalDataOut, CreatePersonalDTO};
use chrono::NaiveDate;
use uuid::Uuid;

#[derive(Serialize)]
struct ApiResponse<T: serde::Serialize> {
    status: String,
    message: String,
    data: Option<T>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProfileDbRecord {
    pub id: String,
    pub date_of_birth: Option<String>,
    pub primary_skill: Option<String>,
    pub skill_to_learn: Option<String>,
    pub bio: Option<String>,
    pub profile_picture_url: Option<String>,
    pub full_name: Option<String>,
    pub role: Option<String>,
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

    // Get profile from profiles table
    match get_user_profile_data(&svc, auth_user.user_id).await {
        Ok(profile_opt) => {
            if let Some(profile) = profile_opt {
                // Convert to PersonalDataOut using your exact DTO structure
                let personal_data = PersonalDataOut {
                    id: Uuid::parse_str(&profile.id).unwrap_or(auth_user.user_id),
                    user_id: auth_user.user_id,
                    date_of_birth: profile.date_of_birth.unwrap_or_default(),
                    primary_skill: profile.primary_skill.unwrap_or_default(),
                    skill_to_learn: profile.skill_to_learn.unwrap_or_default(),
                    bio: profile.bio.unwrap_or_default(),
                    profile_picture_url: profile.profile_picture_url,
                };

                println!("Profile found: {:?}", personal_data);

                HttpResponse::Ok().json(ApiResponse {
                    status: "success".to_string(),
                    message: "Profile retrieved successfully".to_string(),
                    data: Some(personal_data),
                })
            } else {
                println!("No profile found for user {}", auth_user.user_id);
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

    // Validate required fields
    if body.primary_skill.trim().is_empty() {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            status: "error".to_string(),
            message: "Primary skill is required".to_string(),
            data: None,
        });
    }

    if body.skill_to_learn.trim().is_empty() {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            status: "error".to_string(),
            message: "Skill to learn is required".to_string(),
            data: None,
        });
    }

    // Validate and convert date format - allow empty dates
    let iso_date = if body.date_of_birth.trim().is_empty() {
        "".to_string()
    } else {
        // The frontend should already send in YYYY-MM-DD format, but let's be flexible
        match NaiveDate::parse_from_str(&body.date_of_birth, "%Y-%m-%d")
            .or_else(|_| NaiveDate::parse_from_str(&body.date_of_birth, "%d/%m/%Y"))
            .or_else(|_| NaiveDate::parse_from_str(&body.date_of_birth, "%m/%d/%Y"))
        {
            Ok(d) => d.format("%Y-%m-%d").to_string(),
            Err(e) => {
                println!("Invalid date format received: '{}', error: {}", body.date_of_birth, e);
                return HttpResponse::BadRequest().json(ApiResponse::<()> {
                    status: "error".to_string(),
                    message: format!("Invalid date format: '{}'. Use YYYY-MM-DD", body.date_of_birth),
                    data: None,
                });
            }
        }
    };

    let profile_dto = CreatePersonalDTO {
        date_of_birth: iso_date,
        primary_skill: body.primary_skill.trim().to_string(),
        skill_to_learn: body.skill_to_learn.trim().to_string(),
        bio: body.bio.trim().to_string(),
    };

    println!("Processed profile DTO: {:?}", profile_dto);

    match upsert_profile_data(&svc, auth_user.user_id, profile_dto).await {
        Ok(updated_profile) => {
            println!("Profile updated successfully: {:?}", updated_profile);
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
                message: format!("Failed to update profile: {}", e),
                data: None,
            })
        }
    }
}

// Remove the get_user_auth_info function since we're not using it anymore

// Helper function to get profile from profiles table
async fn get_user_profile_data(
    svc: &AuthService,
    user_id: uuid::Uuid,
) -> Result<Option<ProfileDbRecord>, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/rest/v1/profiles", svc.supabase_url);
    
    println!("Getting profile data from: {}", url);
    
    let response = svc.client
        .get(&url)
        .header("apikey", &svc.supabase_service_role_key)
        .header("Authorization", format!("Bearer {}", &svc.supabase_service_role_key))
        .query(&[
            ("id", format!("eq.{}", user_id)),
            ("select", "*".to_string())
        ])
        .send()
        .await?;

    println!("Profile response status: {}", response.status());

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        println!("Failed to get profile: {} - {}", status, error_text);
        return Err(format!("Failed to get profile: {}", error_text).into());
    }

    let profiles: Vec<serde_json::Value> = response.json().await?;
    println!("Profile data: {:?}", profiles);
    
    if let Some(profile_data) = profiles.first() {
        Ok(Some(ProfileDbRecord {
            id: profile_data["id"].as_str().unwrap_or("").to_string(),
            date_of_birth: profile_data["date_of_birth"].as_str().map(|s| s.to_string()),
            primary_skill: profile_data["primary_skill"].as_str().map(|s| s.to_string()),
            skill_to_learn: profile_data["skill_to_learn"].as_str().map(|s| s.to_string()),
            bio: profile_data["bio"].as_str().map(|s| s.to_string()),
            profile_picture_url: profile_data["profile_picture_url"].as_str().map(|s| s.to_string()),
            full_name: profile_data["full_name"].as_str().map(|s| s.to_string()),
            role: profile_data["role"].as_str().map(|s| s.to_string()),
        }))
    } else {
        Ok(None)
    }
}

// Helper function to upsert profile data (insert or update)
async fn upsert_profile_data(
    svc: &AuthService,
    user_id: uuid::Uuid,
    profile_dto: CreatePersonalDTO,
) -> Result<PersonalDataOut, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/rest/v1/profiles", svc.supabase_url);
    
    // Prepare the upsert data - ensure all fields are present
    let upsert_data = serde_json::json!({
        "id": user_id,
        "date_of_birth": if profile_dto.date_of_birth.is_empty() { 
            serde_json::Value::Null 
        } else { 
            serde_json::Value::String(profile_dto.date_of_birth.clone()) 
        },
        "primary_skill": profile_dto.primary_skill,
        "skill_to_learn": profile_dto.skill_to_learn,
        "bio": profile_dto.bio,
    });

    println!("Upserting profile data: {}", serde_json::to_string_pretty(&upsert_data).unwrap_or_default());

    let response = svc.client
        .post(&url)
        .header("apikey", &svc.supabase_service_role_key)
        .header("Authorization", format!("Bearer {}", &svc.supabase_service_role_key))
        .header("Content-Type", "application/json")
        .header("Prefer", "resolution=merge-duplicates,return=representation")
        .json(&upsert_data)
        .send()
        .await?;

    let status = response.status();
    println!("Upsert response status: {}", status);

    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        println!("Upsert failed: {} - {}", status, error_text);
        
        // Try to parse error details for better debugging
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
            println!("Parsed error: {}", serde_json::to_string_pretty(&error_json).unwrap_or_default());
        }
        
        return Err(format!("Failed to upsert profile: {} - {}", status, error_text).into());
    }

    // Try to get the response as JSON
    let response_text = response.text().await?;
    println!("Upsert response body: {}", response_text);

    let updated_profiles: Vec<serde_json::Value> = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse response JSON: {} - Response: {}", e, response_text))?;
    
    if let Some(profile_data) = updated_profiles.first() {
        // Parse the UUID from the response
        let id_str = profile_data["id"].as_str()
            .ok_or("Missing id in profile response")?;
        let parsed_id = uuid::Uuid::parse_str(id_str)
            .map_err(|e| format!("Invalid UUID format for id: {}", e))?;

        let result = PersonalDataOut {
            id: parsed_id,
            user_id: parsed_id, // In profiles table, id is the user_id
            date_of_birth: profile_data["date_of_birth"].as_str().unwrap_or("").to_string(),
            primary_skill: profile_data["primary_skill"].as_str().unwrap_or("").to_string(),
            skill_to_learn: profile_data["skill_to_learn"].as_str().unwrap_or("").to_string(),
            bio: profile_data["bio"].as_str().unwrap_or("").to_string(),
            profile_picture_url: profile_data["profile_picture_url"].as_str().map(|s| s.to_string()),
        };

        println!("Successfully parsed result: {:?}", result);
        Ok(result)
    } else {
        Err("No profile data returned from upsert".into())
    }
}