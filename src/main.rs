// src/main.rs - FIXED VERSION for Railway deployment
mod dtos;
mod services;
mod handlers;
mod repositories;
mod models;
mod middleware;
mod config;

use std::env;
use actix_web::{App, HttpServer, web, middleware::Logger};
use deadpool_postgres::Pool;
use actix_cors::Cors;
use reqwest::Client;
use log::{info, error};
use crate::handlers::profile_handlers::{get_user_profile, update_user_profile};

use crate::handlers::auth_handlers::{
    signup, 
    complete_profile, 
    login, 
    get_skills, 
    test_supabase, 
    get_current_profile
};
use crate::services::auth_services::AuthService;
use crate::handlers::profile_picture_handlers::{
    upload_profile_picture,
    skip_profile_picture, 
    serve_profile_picture,
};
use crate::handlers::post_handlers::{create_post, list_posts};

fn mask_key(k: &str) -> String {
    if k.len() <= 8 { "[REDACTED]".to_string() }
    else { format!("{}***{}", &k[..4], &k[k.len()-4..]) }
}

#[derive(Clone)]
pub struct AppState {
    pub pg_pool: Pool,
    pub supabase_url: String,
    pub supabase_key: String,
    pub http_client: Client,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    dotenv::dotenv().ok();

    let supabase_url = env::var("SUPABASE_URL")
        .expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_SERVICE_ROLE_KEY")
        .expect("SUPABASE_SERVICE_ROLE_KEY must be set");

    info!("Supabase URL: {}", supabase_url);
    info!("Supabase Key: {}", mask_key(&supabase_key));

    let pg_pool = match config::get_pg_pool() {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to create PG pool: {}", e);
            std::process::exit(1);
        }
    };

    let http_client = Client::builder()
        .user_agent("barterup-be/0.1")
        .build()
        .expect("failed to build http client");

    let auth_service = AuthService::new_from_env();
    let auth_data = web::Data::new(auth_service);

    let state = web::Data::new(AppState {
        pg_pool,
        supabase_url: supabase_url.clone(),
        supabase_key: supabase_key.clone(),
        http_client,
    });

    let allowed_origins = env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://127.0.0.1:3000".into());

    // Get port from environment (Railway sets this)
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);
    
    info!("Starting server on {}", bind_address);

    // src/main.rs - FIXED ROUTE REGISTRATION
// ... (other imports and setup code remains same)

HttpServer::new(move || {
    let mut cors = Cors::default()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![
            "authorization", 
            "content-type", 
            "accept",
            "x-requested-with"
        ])
        .supports_credentials()
        .max_age(3600);

    for origin in allowed_origins.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        cors = cors.allowed_origin(origin);
    }

    App::new()
        .wrap(cors)
        .wrap(Logger::default())
        .app_data(state.clone())
        .app_data(auth_data.clone())
        // FIXED: All routes properly registered
        .service(
            web::scope("/auth")
                .service(signup)            // POST /auth/signup
                .service(complete_profile) // POST /auth/complete-profile  
                .service(login)            // POST /auth/login
        )
        .service(
            web::scope("/api")
                .service(get_skills)       // GET /api/skills
                .service(get_user_profile) // GET /api/profile
                .service(update_user_profile) // PUT /api/profile
                .service(get_current_profile) // GET /api/profile (duplicate?)
                .service(create_post)      // POST /api/posts
                .service(list_posts)       // GET /api/posts
        )
        .service(
            web::scope("/api/profile-picture")
                .service(upload_profile_picture) // POST /api/profile-picture/upload
                .service(skip_profile_picture)   // POST /api/profile-picture/skip
                .service(serve_profile_picture)  // GET /api/profile-picture/{user_id}
        )
        .service(test_supabase) // GET /test/supabase
})
.bind(&bind_address)?
.run()
.await
}