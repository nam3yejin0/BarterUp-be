use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::dtos::personal_dtos::CreatePersonalDTO;
use crate::dtos::personal_dtos::PersonalDataOut;

#[derive(Deserialize)]
pub struct SignupIn {
    pub email: String,
    pub password: String,
    pub username: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginIn {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct SessionOut {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub token_type: Option<String>,
}

// NEW DTOs for complete flow

#[derive(Deserialize)]
pub struct CompleteProfileRequest {
    pub email: String,
    pub password: String,
    pub profile: CreatePersonalDTO,
}

#[derive(Serialize)]
pub struct SignupResponse {
    pub user_id: Uuid,
    pub message: String,
    pub next_step: String,
}

#[derive(Serialize)]
pub struct ProfileCompleteResponse {
    pub session: SessionOut,
    pub profile: PersonalDataOut,
    pub message: String,
    pub next_step: String,
}

#[derive(Serialize)]
pub struct LoginWithProfileResponse {
    pub session: SessionOut,
    pub profile: PersonalDataOut,
    pub message: String,
    pub next_step: String,
}

#[derive(Serialize)]
pub struct LoginNoProfileResponse {
    pub session: SessionOut,
    pub message: String,
    pub next_step: String,
}