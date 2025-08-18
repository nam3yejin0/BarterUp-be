use actix_web::{get, post, web, HttpResponse, Responder};
use uuid::Uuid;
use regex::Regex;
use chrono::NaiveDate;
use serde::Serialize;
use crate::models::personal::get_valid_skills;

use crate::dtos::auth::{SignupIn, LoginIn, SessionOut};
use crate::dtos::personal::{CreatePersonalDTO, PersonalDataOut};
use crate::services::auth_services::AuthService;
use crate::middleware::auth_extractor::AuthenticatedUser;
use crate::models::personal::NewPersonal;
use crate::dtos::auth_dtos::CompleteProfileRequest;
use crate::dtos::auth_dtos::LoginWithProfileResponse;
use crate::dtos::auth_dtos::LoginNoProfileResponse;

fn looks_like_email(email: &str) -> bool {
    let re = Regex::new(r"(?i)^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$").unwrap();
    re.is_match(email)
}

#[derive(Serialize)]
struct ApiResponse<T: serde::Serialize> {
    status: String,
    message: String,
    data: Option<T>,
}

#[derive(Serialize)]
struct SkillsResponse {
    skills: Vec<&'static str>,
    total: usize,
}

#[derive(serde::Serialize)]
struct SignupResponse {
    user_id: Uuid,
    message: String,
    next_step: String,
}

#[derive(serde::Serialize)]
struct ProfileCompleteResponse {
    session: SessionOut,
    profile: PersonalDataOut,
    message: String,
    next_step: String,
}

/// POST /auth/signup
/// Step 1: Create account only, no session returned
/// Client redirects to profile creation
#[post("/auth/signup")]
pub async fn signup(
    svc: web::Data<AuthService>,
    body: web::Json<SignupIn>,
) -> impl Responder {
    let email = body.email.trim().to_lowercase();
    
    // Validate email format
    if !looks_like_email(&email) {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            status: "error".to_string(),
            message: "Invalid email format".to_string(),
            data: None,
        });
    }

    // Validate password length
    if body.password.len() < 6 {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            status: "error".to_string(),
            message: "Password must be at least 6 characters long".to_string(),
            data: None,
        });
    }

    let signup_data = SignupIn {
        email: email.clone(),
        password: body.password.clone(),
        username: body.username.clone(),
    };

    match svc.signup_only(signup_data).await {
        Ok(user_id) => {
            let response = SignupResponse {
                user_id,
                message: "Account created successfully. Please complete your profile to continue.".to_string(),
                next_step: "complete_profile".to_string(),
            };

            HttpResponse::Created().json(ApiResponse {
                status: "success".to_string(),
                message: "Account created".to_string(),
                data: Some(response),
            })
        }
        Err(e) => {
            eprintln!("Signup error: {}", e);
            
            // Handle specific Supabase errors
            let error_msg = if e.to_string().contains("already registered") {
                "Email already exists. Please login instead."
            } else {
                "Failed to create account. Please try again."
            };

            HttpResponse::BadRequest().json(ApiResponse::<()> {
                status: "error".to_string(),
                message: error_msg.to_string(),
                data: None,
            })
        }
    }
}

/// POST /auth/complete-profile
/// Step 2: Add profile data and auto-login
/// Returns session + profile data for dashboard redirect
#[post("/auth/complete-profile")]
pub async fn complete_profile(
    svc: web::Data<AuthService>,
    body: web::Json<CompleteProfileRequest>,
) -> impl Responder {
    // Validate all required fields
    if body.email.trim().is_empty() 
        || body.password.trim().is_empty()
        || body.profile.date_of_birth.trim().is_empty()
        || body.profile.primary_skill.trim().is_empty()
        || body.profile.skill_to_learn.trim().is_empty()
        || body.profile.bio.trim().is_empty() {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            status: "error".to_string(),
            message: "All fields are required".to_string(),
            data: None,
        });
    }

    // Parse and validate date
    let parsed_date = match NaiveDate::parse_from_str(&body.profile.date_of_birth, "%d/%m/%Y") {
        Ok(d) => d,
        Err(_) => {
            // Fallback to ISO format
            match NaiveDate::parse_from_str(&body.profile.date_of_birth, "%Y-%m-%d") {
                Ok(d2) => d2,
                Err(_) => {
                    return HttpResponse::BadRequest().json(ApiResponse::<()> {
                        status: "error".to_string(),
                        message: "Invalid date format. Use DD/MM/YYYY".to_string(),
                        data: None,
                    });
                }
            }
        }
    };

    // Validate age (13-120 years)
    let today = chrono::Utc::now().naive_utc().date();
    let min_date = today - chrono::Duration::days(365 * 120);
    let max_date = today - chrono::Duration::days(365 * 13);

    if parsed_date < min_date || parsed_date > max_date {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            status: "error".to_string(),
            message: "Age must be between 13 and 120 years".to_string(),
            data: None,
        });
    }

    // Validate field lengths
    if body.profile.primary_skill.len() > 100 || body.profile.skill_to_learn.len() > 100 {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            status: "error".to_string(),
            message: "Skills must be less than 100 characters each".to_string(),
            data: None,
        });
    }

    if body.profile.bio.len() > 1000 {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            status: "error".to_string(),
            message: "Bio must be less than 1000 characters".to_string(),
            data: None,
        });
    }

    // Step 1: Login to get user_id and session
    let login_data = LoginIn {
        email: body.email.clone(),
        password: body.password.clone(),
    };

    let (session, user_id) = match svc.login_with_user_id(login_data).await {
        Ok((session, user_id)) => (session, user_id),
        Err(e) => {
            eprintln!("Login failed during profile completion: {}", e);
            return HttpResponse::Unauthorized().json(ApiResponse::<()> {
                status: "error".to_string(),
                message: "Invalid credentials or account not activated".to_string(),
                data: None,
            });
        }
    };

    // Step 2: Save profile using the user_id from login response
    let iso_date = parsed_date.format("%Y-%m-%d").to_string();
    let profile_dto = CreatePersonalDTO {
        date_of_birth: iso_date,
        primary_skill: body.profile.primary_skill.clone(),
        skill_to_learn: body.profile.skill_to_learn.clone(),
        bio: body.profile.bio.clone(),
    };

    
    match svc.add_personal_sb(user_id, profile_dto).await {
        Ok(saved_profile) => {
            let response = ProfileCompleteResponse {
                session,
                profile: saved_profile,
                message: "Profile completed successfully! Now you can upload a profile picture.".to_string(),
                next_step: "upload_profile".to_string(), // CHANGED: redirect ke upload profile
            };

            HttpResponse::Created().json(ApiResponse {
                status: "success".to_string(),
                message: "Profile completed and logged in".to_string(),
                data: Some(response),
            })
        }
        Err(e) => {
            eprintln!("Failed to save profile for user {}: {}", user_id, e);
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                status: "error".to_string(),
                message: "Failed to save profile. Please try again.".to_string(),
                data: None,
            })
        }
    }
}

/// GET /api/skills
/// Public endpoint to get available skill options
#[get("/api/skills")]
pub async fn get_skills() -> impl Responder {
    let skills = get_valid_skills();
    let response = SkillsResponse {
        total: skills.len(),
        skills,
    };

    HttpResponse::Ok().json(ApiResponse {
        status: "success".to_string(),
        message: "Skills retrieved successfully".to_string(),
        data: Some(response),
    })
}

/// POST /auth/login
/// For existing users with complete profiles
/// Checks if profile exists and redirects accordingly
#[post("/auth/login")]
pub async fn login(
    svc: web::Data<AuthService>,
    body: web::Json<LoginIn>,
) -> impl Responder {
    let login_data = body.into_inner();

    // Step 1: Authenticate user and get user_id directly from response
    let (session, user_id) = match svc.login_with_user_id(login_data).await {
        Ok((session, user_id)) => (session, user_id),
        Err(e) => {
            eprintln!("Login failed: {}", e);
            return HttpResponse::Unauthorized().json(ApiResponse::<()> {
                status: "error".to_string(),
                message: "Invalid email or password".to_string(),
                data: None,
            });
        }
    };

    // Step 2: Check if user has profile
    match svc.get_user_profile(user_id).await {
        Ok(Some(profile)) => {
            // User has profile - direct to dashboard
            let response = LoginWithProfileResponse {
                session,
                profile,
                message: "Login successful! Welcome back.".to_string(),
                next_step: "dashboard".to_string(),
            };

            HttpResponse::Ok().json(ApiResponse {
                status: "success".to_string(),
                message: "Login successful".to_string(),
                data: Some(response),
            })
        }
        Ok(None) => {
            // User exists but no profile - redirect to profile creation
            let response = LoginNoProfileResponse {
                session,
                message: "Please complete your profile to continue.".to_string(),
                next_step: "complete_profile".to_string(),
            };

            HttpResponse::Ok().json(ApiResponse {
                status: "success".to_string(),
                message: "Profile required".to_string(),
                data: Some(response),
            })
        }
        Err(e) => {
            eprintln!("Failed to check user profile: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                status: "error".to_string(),
                message: "Failed to verify account status".to_string(),
                data: None,
            })
        }
    }
}
// Tambahkan ini ke handlers/auth_handlers.rs

// Add this to src/handlers/auth_handlers.rs

/// GET /api/profile
/// Get current user's profile data (requires authentication)
#[get("/api/profile")]
pub async fn get_current_profile(
    svc: web::Data<AuthService>,
    user: AuthenticatedUser,
) -> impl Responder {
    println!("=== GET PROFILE REQUEST ===");
    println!("User ID: {}", user.user_id);
    
    match svc.get_user_profile(user.user_id).await {
        Ok(Some(profile)) => {
            println!("Profile found for user {}: {:?}", user.user_id, profile);
            HttpResponse::Ok().json(ApiResponse {
                status: "success".to_string(),
                message: "Profile retrieved successfully".to_string(),
                data: Some(profile),
            })
        }
        Ok(None) => {
            println!("No profile found for user {}", user.user_id);
            HttpResponse::NotFound().json(ApiResponse::<()> {
                status: "error".to_string(),
                message: "Profile not found. Please complete your profile first.".to_string(),
                data: None,
            })
        }
        Err(e) => {
            eprintln!("Failed to get profile for user {}: {}", user.user_id, e);
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                status: "error".to_string(),
                message: "Failed to retrieve profile".to_string(),
                data: None,
            })
        }
    }
}

#[get("/test/supabase")]
pub async fn test_supabase(svc: web::Data<AuthService>) -> impl Responder {
    let url = format!("{}/rest/v1/profiles?limit=1", svc.supabase_url);
    
    let resp = svc.client
        .get(&url)
        .header("apikey", &svc.supabase_anon_key)
        .header("Authorization", format!("Bearer {}", &svc.supabase_service_role_key))
        .send()
        .await;

    match resp {
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "supabase_status": status.as_u16(),
                "body": body
            }))
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": format!("Supabase connection failed: {}", e)
            }))
        }
    }
}