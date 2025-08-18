// src/repositories/profile_supabase_repo.rs
use crate::models::personal::{NewPersonal, Personal}; // sesuaikan path
use crate::dtos::personal::CreatePersonalDTO;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;
use std::env;
use urlencoding::encode;

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("supabase error: {0}")]
    Supabase(String),
    #[error("not found")]
    NotFound,
    #[error("other: {0}")]
    Other(String),
}

/// Repository untuk table `profiles` via Supabase (PostgREST)
#[derive(Clone)]
pub struct ProfileSupabaseRepo {
    client: Client,
    base_rest_url: String,      // e.g. https://xyz.supabase.co/rest/v1
    service_role_key: String,   // SUPABASE_SERVICE_ROLE_KEY (server-only)
    anon_key: Option<String>,   // optional
}

impl ProfileSupabaseRepo {
    /// create from env vars (helper). Panik kalau service role key tidak ada.
    pub fn new_from_env() -> Self {
        let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL required");
        let rest = if supabase_url.ends_with("/rest/v1") {
            supabase_url.trim_end_matches('/').to_string()
        } else {
            format!("{}/rest/v1", supabase_url.trim_end_matches('/'))
        };

        let service_role_key =
            env::var("SUPABASE_SERVICE_ROLE_KEY").expect("SUPABASE_SERVICE_ROLE_KEY required");
        let anon_key = env::var("SUPABASE_ANON_KEY").ok();

        Self {
            client: Client::new(),
            base_rest_url: rest,
            service_role_key,
            anon_key,
        }
    }

    fn profiles_url(&self) -> String {
        format!("{}/profiles", self.base_rest_url.trim_end_matches('/'))
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        // apikey is sometimes required by Supabase; include anon_key if available
        if let Some(ref key) = self.anon_key {
            headers.insert("apikey", HeaderValue::from_str(key).unwrap());
        }
        // service role key in Authorization: Bearer <key>
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.service_role_key)).unwrap(),
        );
        headers
    }

    /// Upsert / create profile using PostgREST upsert via Prefer header.
    /// Returns the saved Personal (first row of representation).
    pub async fn upsert_profile(
        &self,
        user_id: Uuid,
        dto: CreatePersonalDTO, // date_of_birth expected ISO YYYY-MM-DD
    ) -> Result<Personal, RepoError> {
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

        let url = self.profiles_url();
        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .header("Prefer", "resolution=merge-duplicates,return=representation")
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(RepoError::Supabase(format!(
                "{} -> {}",
                status.as_u16(),
                text
            )));
        }

        // Expect array with single object
        let arr: Vec<Value> = serde_json::from_str(&text)?;
        let first = arr.into_iter().next().ok_or(RepoError::Other(
            "empty response from upsert".to_string(),
        ))?;

        // Map value -> Personal (try serde direct mapping)
        let personal: Personal = serde_json::from_value(first)?;
        Ok(personal)
    }

    /// Get profile by user id (id = primary key referencing auth.users.id)
    pub async fn get_by_user_id(&self, user_id: Uuid) -> Result<Personal, RepoError> {
        // PostgREST filter: ?id=eq.<uuid>&select=*
        // url encode user_id just in case
        let url = format!(
            "{}?id=eq.{}&select=*",
            self.profiles_url(),
            encode(&user_id.to_string())
        );

        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(RepoError::Supabase(format!(
                "{} -> {}",
                status.as_u16(),
                text
            )));
        }

        let arr: Vec<Personal> = serde_json::from_str(&text)?;
        arr.into_iter().next().ok_or(RepoError::NotFound)
    }

    /// Get role value for user (returns Ok(Some(role)) or Ok(None) if not exist)
    pub async fn get_role_by_user_id(&self, user_id: Uuid) -> Result<Option<String>, RepoError> {
        let url = format!(
            "{}?id=eq.{}&select=role",
            self.profiles_url(),
            encode(&user_id.to_string())
        );

        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(RepoError::Supabase(format!(
                "{} -> {}",
                status.as_u16(),
                text
            )));
        }

        // parse array -> role field
        let arr: Vec<Value> = serde_json::from_str(&text)?;
        if let Some(first) = arr.into_iter().next() {
            if let Some(role_val) = first.get("role").and_then(|r| r.as_str()) {
                return Ok(Some(role_val.to_string()));
            }
        }
        Ok(None)
    }

    /// Delete profile by user id. Returns true when deleted (i.e. success & not 404)
    pub async fn delete_by_user_id(&self, user_id: Uuid) -> Result<bool, RepoError> {
        let url = format!("{}?id=eq.{}", self.profiles_url(), encode(&user_id.to_string()));
        let resp = self
            .client
            .delete(&url)
            .headers(self.headers())
            .header("Prefer", "return=representation")
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if status.is_success() {
            // If deletion succeeded, PostgREST returns array of deleted rows (if prefer return)
            // We'll treat success as true.
            return Ok(true);
        } else {
            return Err(RepoError::Supabase(format!(
                "{} -> {}",
                status.as_u16(),
                text
            )));
        }
    }
}
