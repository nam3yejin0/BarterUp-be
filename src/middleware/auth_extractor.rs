// src/middleware/auth_extractor.rs - SUPER SIMPLE untuk projek sekolah
use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use actix_web::error::ErrorUnauthorized;
use futures::future::{ready, Ready};
use uuid::Uuid;
use base64::Engine; // Add this import to bring the Engine trait into scope

/// Hasil extractor - user yang sudah terautentikasi
pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

impl FromRequest for AuthenticatedUser {
    type Error = Error;
    type Future = Ready<Result<AuthenticatedUser, Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // Ambil header Authorization
        let auth_header = match req.headers().get("Authorization") {
            Some(header) => match header.to_str() {
                Ok(h) => h,
                Err(_) => return ready(Err(ErrorUnauthorized("Invalid header format"))),
            },
            None => return ready(Err(ErrorUnauthorized("Missing Authorization header"))),
        };

        // Cek format Bearer token
        if !auth_header.starts_with("Bearer ") {
            return ready(Err(ErrorUnauthorized("Invalid auth header format")));
        }

        let token = auth_header.trim_start_matches("Bearer ").trim();
        
        println!("=== AUTH DEBUG ===");
        println!("Token received (first 50 chars): {}", &token[..std::cmp::min(token.len(), 50)]);

        // SUPER SIMPLE: Extract user_id from JWT payload tanpa validasi signature
        // HANYA UNTUK PROJEK SEKOLAH - TIDAK AMAN!
        match extract_user_id_from_jwt(token) {
            Ok(user_id) => {
                println!("Auth successful for user: {}", user_id);
                ready(Ok(AuthenticatedUser { user_id }))
            }
            Err(e) => {
                println!("Auth failed: {}", e);
                ready(Err(ErrorUnauthorized("Invalid token")))
            }
        }
    }
}

// SUPER SIMPLE JWT parser - hanya ambil user ID dari payload
// TIDAK VALIDASI SIGNATURE - HANYA UNTUK DEVELOPMENT/SEKOLAH!
fn extract_user_id_from_jwt(token: &str) -> Result<Uuid, String> {
    // JWT format: header.payload.signature
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT format".to_string());
    }

    // Decode payload (bagian ke-2) - JWT menggunakan base64url tanpa padding
    let payload = parts[1];
    
    println!("Raw payload part: {}", payload);
    
    // Gunakan URL_SAFE_NO_PAD dan JANGAN tambahkan padding manual
    match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload) {
        Ok(decoded) => {
            let payload_str = String::from_utf8(decoded).map_err(|e| format!("UTF8 error: {}", e))?;
            println!("Decoded payload: {}", payload_str);
            
            // Parse JSON untuk ambil 'sub' field (user ID)
            let json: serde_json::Value = serde_json::from_str(&payload_str)
                .map_err(|e| format!("JSON parse error: {}", e))?;
            
            let user_id_str = json["sub"].as_str()
                .ok_or("Missing 'sub' field in token")?;
            
            Uuid::parse_str(user_id_str)
                .map_err(|e| format!("Invalid UUID: {}", e))
        }
        Err(e) => {
            println!("Base64 decode failed, trying with standard decoder...");
            // Fallback: coba dengan standard base64 jika URL_SAFE_NO_PAD gagal
            match base64::engine::general_purpose::STANDARD.decode(payload) {
                Ok(decoded) => {
                    let payload_str = String::from_utf8(decoded).map_err(|e| format!("UTF8 error: {}", e))?;
                    println!("Decoded payload (standard): {}", payload_str);
                    
                    let json: serde_json::Value = serde_json::from_str(&payload_str)
                        .map_err(|e| format!("JSON parse error: {}", e))?;
                    
                    let user_id_str = json["sub"].as_str()
                        .ok_or("Missing 'sub' field in token")?;
                    
                    Uuid::parse_str(user_id_str)
                        .map_err(|e| format!("Invalid UUID: {}", e))
                }
                Err(e2) => Err(format!("Both base64 decoders failed: {} and {}", e, e2))
            }
        }
    }
}