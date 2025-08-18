use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{NaiveDateTime, Utc};

/// Representasi row `profiles` / `users` yang kita pakai di aplikasi.
/// Catatan: password tidak disimpan di sini — Supabase Auth meng-handle password.
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,                   // sama dengan auth.users.id (supabase)
    pub email: String,              // dari auth.users.email
    pub username: Option<String>,   // optional
    pub full_name: Option<String>,  // dari profile
    pub role: String,               // "user" (server-set)
    pub is_active: bool,            // optional flag
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

/// Struct untuk membuat / upsert profile (insert into profiles table)
#[derive(Debug, Serialize, Deserialize)]
pub struct NewUser {
    pub id: Uuid,                   // user_id (from auth)
    pub email: String,
    pub username: Option<String>,
    pub full_name: Option<String>,
    pub role: String,               // set "user" server-side
}

/// Versi yang dikirim ke client (redacted)
#[derive(Debug, Serialize, Deserialize)]
pub struct UserPublic {
    pub id: Uuid,
    pub username: Option<String>,
    pub full_name: Option<String>,
    pub role: String,
}

/// JWT claims yang biasa ada di token Supabase (disesuaikan)
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    /// subject / user id
    pub sub: String,
    pub aud: Option<String>,
    pub exp: Option<u64>,
    pub iat: Option<u64>,
    pub role: Option<String>,    // kadang Supabase menyertakan role di klaim
    pub email: Option<String>,
}
