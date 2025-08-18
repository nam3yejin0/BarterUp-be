// src/services/auth_services.rs - Fixed version
use std::env;
use chrono::NaiveDate;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::dtos::auth::{SignupIn, LoginIn, SessionOut};
use crate::dtos::personal::{CreatePersonalDTO, PersonalDataOut};

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("supabase error: {0}")]
    Supabase(String),
    #[error("invalid token")]
    InvalidToken,
    #[error("parse uuid error")]
    UuidError(#[from] uuid::Error),
    #[error("user not found")]
    UserNotFound,
    #[error("profile not found")]
    ProfileNotFound,
    #[error("other: {0}")]
    Other(String),
}

#[derive(Clone)]
pub struct AuthService {
    pub client: reqwest::Client,
    pub supabase_url: String,
    pub supabase_anon_key: String,
    pub supabase_service_role_key: String,
}

impl AuthService {
    pub fn new_from_env() -> Self {
        let supabase_url = env::var("SUPABASE_URL")
            .expect("SUPABASE_URL is required")
            .trim()
            .to_string();

        let supabase_anon_key = env::var("SUPABASE_ANON_KEY")
            .unwrap_or_default()
            .trim()
            .to_string();

        let supabase_service_role_key = env::var("SUPABASE_SERVICE_ROLE_KEY")
            .expect("SUPABASE_SERVICE_ROLE_KEY required")
            .trim()
            .to_string();

        Self {
            client: reqwest::Client::new(),
            supabase_url,
            supabase_anon_key,
            supabase_service_role_key,
        }
    }

    pub async fn signup_only(&self, input: SignupIn) -> Result<Uuid, AuthError> {
        #[derive(Serialize)]
        struct Body<'a> {
            email: &'a str,
            password: &'a str,
        }

        let email_trimmed = input.email.trim();
        let body = Body {
            email: email_trimmed,
            password: &input.password,
        };

        let url = format!("{}/auth/v1/signup", self.supabase_url.trim_end_matches('/'));
        
        let resp = self
            .client
            .post(&url)
            .header("apikey", &self.supabase_anon_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(msg) = error_json.get("msg").or_else(|| error_json.get("message")) {
                    return Err(AuthError::Supabase(msg.as_str().unwrap_or("Signup failed").to_string()));
                }
            }
            return Err(AuthError::Supabase(format!("signup failed: {} {}", status, text)));
        }

        let json_val: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| AuthError::Supabase(format!("invalid json: {}", e)))?;

        let user_id_str = json_val
            .get("user")
            .and_then(|u| u.get("id"))
            .or_else(|| json_val.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| AuthError::Supabase("signup returned no user id".to_string()))?;

        let user_id = Uuid::parse_str(user_id_str)?;
        Ok(user_id)
    }

    /// Update profile picture URL for user - CRITICAL METHOD
    pub async fn update_profile_picture(
        &self,
        user_id: Uuid,
        profile_picture_url: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/rest/v1/profiles", self.supabase_url);
        
        let update_data = serde_json::json!({
            "profile_picture_url": profile_picture_url
        });

        println!("=== UPDATE PROFILE PICTURE DATABASE ===");
        println!("User ID: {}", user_id);
        println!("URL: {}", url);
        println!("Update data: {}", update_data);

        let response = self.client
            .patch(&url)
            .header("apikey", &self.supabase_service_role_key)
            .header("Authorization", format!("Bearer {}", &self.supabase_service_role_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=minimal")
            // CRITICAL: Use 'id' not 'user_id' for profiles table
            .query(&[("id", format!("eq.{}", user_id))])
            .json(&update_data)
            .send()
            .await?;

        let status = response.status();
        println!("Database update response status: {}", status);

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            println!("Database update error: {}", error_text);
            return Err(format!("Failed to update profile picture: {} - {}", status, error_text).into());
        }

        println!("Profile picture URL updated in database successfully!");
        Ok(())
    }

    /// Get user profile with profile picture
    pub async fn get_user_profile_with_picture(
        &self,
        user_id: Uuid,
    ) -> Result<Option<crate::dtos::personal::PersonalDataOut>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/rest/v1/personals", self.supabase_url);
        
        let response = self.client
            .get(&url)
            .header("apikey", &self.supabase_anon_key)
            .header("Authorization", format!("Bearer {}", &self.supabase_service_role_key))
            .query(&[("user_id", format!("eq.{}", user_id)), ("select", "*".to_string())])  // FIXED: convert to String
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch profile: {}", response.status()).into());
        }

        let profiles: Vec<serde_json::Value> = response.json().await?;
        
        if let Some(profile_data) = profiles.first() {
            let profile_out = crate::dtos::personal::PersonalDataOut {
                id: serde_json::from_value(profile_data["id"].clone())?,
                user_id: serde_json::from_value(profile_data["user_id"].clone())?,
                date_of_birth: profile_data["date_of_birth"].as_str().unwrap_or("").to_string(),
                primary_skill: profile_data["primary_skill"].as_str().unwrap_or("").to_string(),
                skill_to_learn: profile_data["skill_to_learn"].as_str().unwrap_or("").to_string(),
                bio: profile_data["bio"].as_str().unwrap_or("").to_string(),
                profile_picture_url: profile_data["profile_picture_url"].as_str().map(|s| s.to_string()),
            };
            Ok(Some(profile_out))
        } else {
            Ok(None)
        }
    }

    // Simplified login - returns session + user_id directly from response
    pub async fn login_with_user_id(&self, input: LoginIn) -> Result<(SessionOut, Uuid), AuthError> {
        #[derive(Serialize)]
        struct LoginBody<'a> {
            email: &'a str,
            password: &'a str,
        }

        #[derive(Deserialize)]
        struct TokenResp {
            access_token: String,
            refresh_token: Option<String>,
            expires_in: Option<i64>,
            token_type: Option<String>,
            user: Option<UserInfo>, // Add user info from response
        }

        #[derive(Deserialize)]
        struct UserInfo {
            id: String,
        }

        let body = LoginBody {
            email: &input.email,
            password: &input.password,
        };

        let url = format!(
            "{}/auth/v1/token?grant_type=password",
            self.supabase_url.trim_end_matches('/')
        );

        let resp = self
            .client
            .post(&url)
            .header("apikey", &self.supabase_anon_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();

        if status != StatusCode::OK {
            return Err(AuthError::Supabase(format!(
                "login failed: {} {}",
                status,
                txt
            )));
        }

        let tr: TokenResp = serde_json::from_str(&txt)
            .map_err(|e| AuthError::Supabase(format!("invalid json in login response: {}", e)))?;

        // Extract user_id from login response instead of JWT
        let user_id = if let Some(user) = tr.user {
            Uuid::parse_str(&user.id)?
        } else {
            return Err(AuthError::Supabase("No user info in login response".to_string()));
        };

        let session = SessionOut {
            access_token: tr.access_token,
            refresh_token: tr.refresh_token,
            expires_in: tr.expires_in,
            token_type: tr.token_type,
        };

        Ok((session, user_id))
    }

    // Keep the old method for compatibility, but use the new one internally
    pub async fn login_sb(&self, input: LoginIn) -> Result<SessionOut, AuthError> {
        let (session, _user_id) = self.login_with_user_id(input).await?;
        Ok(session)
    }

    pub async fn add_personal_sb(
        &self,
        user_id: Uuid,
        dto: CreatePersonalDTO,
    ) -> Result<PersonalDataOut, AuthError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            id: &'a str,
            date_of_birth: &'a str,
            primary_skill: &'a str,
            skill_to_learn: &'a str,
            bio: &'a str,
            role: &'a str,
        }

        let payload = Payload {
            id: &user_id.to_string(),
            date_of_birth: &dto.date_of_birth,
            primary_skill: &dto.primary_skill,
            skill_to_learn: &dto.skill_to_learn,
            bio: &dto.bio,
            role: "user",
        };

        let url = format!("{}/rest/v1/profiles", self.supabase_url.trim_end_matches('/'));

        let resp = self
        .client
        .post(&url)
        // gunakan SERVICE ROLE KEY untuk apikey (server->supabase)
        .header("apikey", &self.supabase_service_role_key)
        // dan juga sebagai Authorization Bearer (server-only)
        .header("Authorization", format!("Bearer {}", &self.supabase_service_role_key))
        .header("Content-Type", "application/json")
        .header("Prefer", "resolution=merge-duplicates,return=representation")
        .json(&payload)
        .send()
        .await?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(AuthError::Supabase(format!(
                "add_personal failed: {} - Response: {}",
                status,
                text
            )));
        }

        let arr: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| AuthError::Supabase(format!("invalid json: {} - Body: {}", e, text)))?;
        
        let first = arr
            .as_array()
            .and_then(|a| a.get(0))
            .ok_or_else(|| AuthError::Supabase(format!("invalid response from profiles upsert: {}", text)))?;

        let out = PersonalDataOut {
            id: Uuid::parse_str(first.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                AuthError::Supabase("missing id in upsert response".into())
            })?)?,
            user_id: Uuid::parse_str(
                first.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AuthError::Supabase("missing user id".into()))?,
            )?,
            date_of_birth: first
                .get("date_of_birth")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            primary_skill: first
                .get("primary_skill")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            skill_to_learn: first
                .get("skill_to_learn")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            bio: first
                .get("bio")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            profile_picture_url: first  // FIXED: Add profile_picture_url field
                .get("profile_picture_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        Ok(out)
    }

    pub async fn get_user_profile(&self, user_id: Uuid) -> Result<Option<PersonalDataOut>, AuthError> {
        let url = format!(
            "{}/rest/v1/profiles?id=eq.{}&select=*",
            self.supabase_url.trim_end_matches('/'),
            user_id
        );

        let resp = self
            .client
            .get(&url)
            .header("apikey", &self.supabase_anon_key)
            .header("Authorization", format!("Bearer {}", &self.supabase_service_role_key))
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(AuthError::Supabase(format!(
                "get_user_profile failed: {} {}",
                status,
                text
            )));
        }

        let arr: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| AuthError::Supabase(format!("invalid json: {}", e)))?;

        let profiles = arr.as_array().ok_or_else(|| {
            AuthError::Supabase("expected array response".into())
        })?;

        if profiles.is_empty() {
            return Ok(None);
        }

        let profile = &profiles[0];
        
        let out = PersonalDataOut {
            id: Uuid::parse_str(profile.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                AuthError::Supabase("missing id in profile response".into())
            })?)?,
            user_id: Uuid::parse_str(
                profile.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AuthError::Supabase("missing user id".into()))?,
            )?,
            date_of_birth: profile
                .get("date_of_birth")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            primary_skill: profile
                .get("primary_skill")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            skill_to_learn: profile
                .get("skill_to_learn")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            bio: profile
                .get("bio")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            profile_picture_url: profile  // FIXED: Add profile_picture_url field
                .get("profile_picture_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        Ok(Some(out))
    }

    pub async fn is_role_user(&self, user_id: Uuid) -> Result<bool, AuthError> {
        let url = format!(
            "{}/rest/v1/profiles?id=eq.{}&select=role",
            self.supabase_url.trim_end_matches('/'),
            user_id
        );

        let resp = self
            .client
            .get(&url)
            .header("apikey", &self.supabase_anon_key)
            .header("Authorization", format!("Bearer {}", &self.supabase_service_role_key))
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(AuthError::Supabase(format!(
                "is_role_user failed: {} {}",
                status,
                text
            )));
        }

        let arr: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| AuthError::Supabase(format!("invalid json: {}", e)))?;

        let role = arr
            .as_array()
            .and_then(|a| a.get(0))
            .and_then(|v| v.get("role"))
            .and_then(|r| r.as_str())
            .unwrap_or("");

        Ok(role == "user")
    }
}