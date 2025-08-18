use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// DTO yang dikirim frontend (FE menyimpan dateOfBirth sebagai "DD/MM/YYYY")
#[derive(Deserialize)]
pub struct CreatePersonalDTO {
    /// dari FE: "DD/MM/YYYY"
    pub date_of_birth: String,
    pub primary_skill: String,
    pub skill_to_learn: String,
    pub bio: String,
}

/// DTO yang dikembalikan ke client setelah tersimpan
#[derive(Serialize, Debug)]
pub struct PersonalDataOut {
    pub id: Uuid,
    pub user_id: Uuid,
    pub date_of_birth: String, // ISO "YYYY-MM-DD"
    pub primary_skill: String,
    pub skill_to_learn: String,
    pub bio: String,
    pub profile_picture_url: Option<String>, // ADDED: Profile picture URL    
    // tambahan field seperti created_at bisa ditambahkan
}
