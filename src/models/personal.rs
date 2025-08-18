// src/models/personal.rs - Update validation

use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{NaiveDate, NaiveDateTime};

// Valid skill options (matching your frontend)
const VALID_SKILLS: &[&str] = &[
    "Music",
    "Art", 
    "Cooking",
    "Photography",
    "Design",
    "Programming",
    "Writing",
    "Fitness",
    "Gardening"
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Personal {
    pub id: Uuid,
    pub user_id: Uuid,
    pub date_of_birth: NaiveDate,
    pub primary_skill: String,
    pub skill_to_learn: String,
    pub bio: String,
    pub profile_picture_url: Option<String>, // ADDED: URL ke gambar profile    
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPersonal {
    pub user_id: Uuid,
    pub date_of_birth: NaiveDate,
    pub primary_skill: String,
    pub skill_to_learn: String,
    pub bio: String,
    pub profile_picture_url: Option<String>, // ADDED: URL ke gambar profile    
}

impl Personal {
    pub fn validate(&self) -> Result<(), String> {
        // Age validation (13-120 years)
        let today = chrono::Utc::now().naive_utc().date();
        let min_date = today - chrono::Duration::days(365 * 120); // 120 years ago
        let max_date = today - chrono::Duration::days(365 * 13);  // 13 years ago

        if self.date_of_birth < min_date || self.date_of_birth > max_date {
            return Err("Age must be between 13-120 years".to_string());
        }

        // Skill validation
        if !VALID_SKILLS.contains(&self.primary_skill.as_str()) {
            return Err("Invalid primary skill. Please select from available options.".to_string());
        }

        if !VALID_SKILLS.contains(&self.skill_to_learn.as_str()) {
            return Err("Invalid skill to learn. Please select from available options.".to_string());
        }

        // Skills can't be the same
        if self.primary_skill == self.skill_to_learn {
            return Err("Primary skill and skill to learn cannot be the same.".to_string());
        }

        // Bio validation
        if self.bio.trim().is_empty() {
            return Err("Bio cannot be empty".to_string());
        }

        if self.bio.len() < 10 {
            return Err("Bio must be at least 10 characters long".to_string());
        }

        if self.bio.len() > 1000 {
            return Err("Bio must be less than 1000 characters".to_string());
        }

        Ok(())
    }

    pub fn age_years(&self) -> i32 {
        let today = chrono::Utc::now().naive_utc().date();
        let age = today.signed_duration_since(self.date_of_birth);
        (age.num_days() / 365) as i32
    }
    // ADDED: Method untuk update profile picture
    pub fn update_profile_picture(&mut self, picture_url: Option<String>) {
        self.profile_picture_url = picture_url;
    }
}

impl NewPersonal {
    pub fn validate(&self) -> Result<(), String> {
        // Age validation
        let today = chrono::Utc::now().naive_utc().date();
        let min_date = today - chrono::Duration::days(365 * 120); // 120 years ago
        let max_date = today - chrono::Duration::days(365 * 13);  // 13 years ago

        if self.date_of_birth < min_date || self.date_of_birth > max_date {
            return Err("Invalid date of birth. Age must be between 13-120 years.".to_string());
        }

        // Skill validation
        if !VALID_SKILLS.contains(&self.primary_skill.as_str()) {
            return Err("Invalid primary skill. Please select from available options.".to_string());
        }

        if !VALID_SKILLS.contains(&self.skill_to_learn.as_str()) {
            return Err("Invalid skill to learn. Please select from available options.".to_string());
        }

        // Skills can't be the same
        if self.primary_skill == self.skill_to_learn {
            return Err("Primary skill and skill to learn cannot be the same.".to_string());
        }

        // Bio validation
        if self.bio.trim().is_empty() {
            return Err("Bio cannot be empty".to_string());
        }

        if self.bio.len() < 10 {
            return Err("Bio must be at least 10 characters long".to_string());
        }

        if self.bio.len() > 1000 {
            return Err("Bio must be less than 1000 characters".to_string());
        }

        Ok(())
    }
}

// Helper function to get valid skills (for API endpoints)
pub fn get_valid_skills() -> Vec<&'static str> {
    VALID_SKILLS.to_vec()
}

// Helper function to validate skill
pub fn is_valid_skill(skill: &str) -> bool {
    VALID_SKILLS.contains(&skill)
}