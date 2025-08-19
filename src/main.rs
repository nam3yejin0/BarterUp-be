// src/main.rs - FIXED VERSION with proper routing
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
            // Auth routes (no /api prefix)
            .service(signup)
            .service(complete_profile)
            .service(login)
            .service(get_skills)
            .service(test_supabase)
            // Profile management routes
            .service(get_user_profile)      // GET /api/profile
            .service(update_user_profile)   // PUT /api/profile
            // Profile routes
            .service(upload_profile_picture)
            .service(skip_profile_picture)
            .service(serve_profile_picture)
            .service(get_current_profile)
            // Posts routes - FIXED: Move outside of scope and add /api prefix
            .service(
                web::scope("/api")
                    .service(create_post)  // This becomes /api/posts
                    .service(list_posts)   // This becomes /api/posts
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}