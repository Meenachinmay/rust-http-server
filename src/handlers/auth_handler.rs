use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info};

use crate::auth::jwt::generate_token;

#[derive(Deserialize)]
pub struct AuthRequest {
    email: String,
}

pub async fn authenticate(auth_req: web::Json<AuthRequest>) -> impl Responder {
    info!("Authentication request for email: {}", auth_req.email);

    match generate_token(auth_req.email.clone()) {
        Ok(token) => HttpResponse::Ok().json(json!({
            "token": token,
            "expires_in": 60
        })),
        Err(e) => {
            error!("Token generation failed: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to generate token"
            }))
        }
    }
}