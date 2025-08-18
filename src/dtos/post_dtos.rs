use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreatePostDTO {
    pub content: String,
    pub image_url: Option<String>, // optional, cocok dengan schema
}

// Add the missing PostOut struct
#[derive(Debug, Serialize, Deserialize)]
pub struct PostOut {
    pub id: String,
    pub user_id: Option<String>,
    pub content: Option<String>,
    pub image_url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}