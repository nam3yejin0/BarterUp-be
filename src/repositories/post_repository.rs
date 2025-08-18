// src/repositories/post_repository.rs - Enhanced version with better profile joins

use reqwest::Client;
use serde_json::json;
use uuid::Uuid;
use crate::dtos::post_dtos::{CreatePostDTO, PostOut};

pub struct PostRepository;

#[derive(serde::Deserialize, Debug)]
pub struct PostWithProfile {
    pub id: String,
    pub user_id: String,
    pub content: Option<String>,
    pub image_url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    // Profile data joined from profiles table
    pub profiles: Option<ProfileData>,
}

#[derive(serde::Deserialize, Debug)]
pub struct ProfileData {
    pub full_name: Option<String>,
    pub primary_skill: Option<String>,
    pub bio: Option<String>,
    pub profile_picture_url: Option<String>,
    pub role: Option<String>,
}

impl PostRepository {
    pub async fn create_post(
        supabase_url: &str,
        service_key: &str,
        client: &Client,
        user_id: Uuid,
        post_data: CreatePostDTO,
    ) -> Result<PostOut, Box<dyn std::error::Error>> {
        let url = format!("{}/rest/v1/posts", supabase_url);
        
        let payload = json!({
            "user_id": user_id,
            "content": post_data.content,
            "image_url": post_data.image_url
        });

        println!("Creating post with payload: {}", payload);

        let response = client
            .post(&url)
            .header("apikey", service_key)
            .header("Authorization", format!("Bearer {}", service_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        
        println!("Post creation response status: {}", status);
        println!("Post creation response body: {}", body);

        if !status.is_success() {
            return Err(format!("Failed to create post: {} - {}", status, body).into());
        }

        let posts: Vec<PostOut> = serde_json::from_str(&body)?;
        posts.into_iter().next()
            .ok_or_else(|| "No post returned from creation".into())
    }

    /// List posts with joined profile data
    pub async fn list_posts_with_profiles(
        supabase_url: &str,
        service_key: &str,
        client: &Client,
        limit: u32,
    ) -> Result<Vec<PostWithProfile>, Box<dyn std::error::Error>> {
        // Enhanced query to get profile data including full_name
        // Note: The profiles table uses 'id' as the primary key that references auth.users.id
        let url = format!(
            "{}/rest/v1/posts?select=*,profiles!posts_user_id_fkey(full_name,primary_skill,bio,profile_picture_url,role)&order=created_at.desc&limit={}",
            supabase_url, limit
        );

        println!("Fetching posts with profiles from: {}", url);

        let response = client
            .get(&url)
            .header("apikey", service_key)
            .header("Authorization", format!("Bearer {}", service_key))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        
        println!("Posts response status: {}", status);
        println!("Posts response body (first 500 chars): {}", 
                if body.len() > 500 { &body[..500] } else { &body });

        if !status.is_success() {
            println!("Profile join failed, trying alternative query...");
            
            // Alternative: Try without explicit foreign key reference
            let alt_url = format!(
                "{}/rest/v1/posts?select=*,profiles(full_name,primary_skill,bio,profile_picture_url,role)&order=created_at.desc&limit={}",
                supabase_url, limit
            );
            
            println!("Trying alternative URL: {}", alt_url);
            
            let alt_response = client
                .get(&alt_url)
                .header("apikey", service_key)
                .header("Authorization", format!("Bearer {}", service_key))
                .send()
                .await?;

            let alt_status = alt_response.status();
            let alt_body = alt_response.text().await?;
            
            println!("Alternative response status: {}", alt_status);
            println!("Alternative response body (first 500 chars): {}", 
                    if alt_body.len() > 500 { &alt_body[..500] } else { &alt_body });

            if !alt_status.is_success() {
                return Err(format!("Failed to fetch posts: {} - {}", alt_status, alt_body).into());
            }

            let posts: Vec<PostWithProfile> = serde_json::from_str(&alt_body)
                .map_err(|e| format!("Failed to parse posts response: {} - Body: {}", e, alt_body))?;
            
            return Ok(posts);
        }

        let posts: Vec<PostWithProfile> = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse posts response: {} - Body: {}", e, body))?;
        
        Ok(posts)
    }

    /// Enhanced method to get posts for a specific user with their profile
    pub async fn get_user_posts_with_profile(
        supabase_url: &str,
        service_key: &str,
        client: &Client,
        user_id: Uuid,
        limit: u32,
    ) -> Result<Vec<PostWithProfile>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/rest/v1/posts?user_id=eq.{}&select=*,profiles(full_name,primary_skill,bio,profile_picture_url,role)&order=created_at.desc&limit={}",
            supabase_url, user_id, limit
        );

        println!("Fetching user posts with profile from: {}", url);

        let response = client
            .get(&url)
            .header("apikey", service_key)
            .header("Authorization", format!("Bearer {}", service_key))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        
        if !status.is_success() {
            return Err(format!("Failed to fetch user posts: {} - {}", status, body).into());
        }

        let posts: Vec<PostWithProfile> = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse user posts response: {} - Body: {}", e, body))?;
        
        Ok(posts)
    }

    /// Fallback method for basic posts (keeping for compatibility)
    pub async fn list_posts(
        supabase_url: &str,
        service_key: &str,
        client: &Client,
        limit: u32,
    ) -> Result<Vec<PostOut>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/rest/v1/posts?order=created_at.desc&limit={}",
            supabase_url, limit
        );

        let response = client
            .get(&url)
            .header("apikey", service_key)
            .header("Authorization", format!("Bearer {}", service_key))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        
        if !status.is_success() {
            return Err(format!("Failed to fetch posts: {} - {}", status, body).into());
        }

        let posts: Vec<PostOut> = serde_json::from_str(&body)?;
        Ok(posts)
    }
}