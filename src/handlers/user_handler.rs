use actix_web::{web, HttpResponse, Responder};
use uuid::Uuid;
use crate::models::user::CreateUserRequest;
use crate::repositories::user_repository::UserRepository;
use tracing::{error, info, instrument};
use serde_json::json;  // Add this import for json! macro

#[instrument(skip(repo, user))]
pub async fn create_user(
    repo: web::Data<UserRepository>,
    user: web::Json<CreateUserRequest>,
) -> impl Responder {
    info!("Attempting to create user with email: {}", user.email);

    match repo.create_user(user.into_inner()).await {
        Ok(created_user) => {
            info!("Successfully created user with id: {}", created_user.uid);
            HttpResponse::Created().json(created_user)
        },
        Err(err) => {
            error!("Failed to create user: {:?}", err);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to create user",
                "details": err.to_string()
            }))
        }
    }
}

#[instrument(skip(repo))]
pub async fn get_user(
    repo: web::Data<UserRepository>,
    id: web::Path<Uuid>,
) -> impl Responder {
    info!("Attempting to fetch user with id: {}", id);

    match repo.get_user_by_id(id.into_inner()).await {
        Ok(Some(user)) => {
            info!("Successfully retrieved user");
            HttpResponse::Ok().json(user)
        },
        Ok(None) => {
            info!("User not found");
            HttpResponse::NotFound().json(json!({
                "error": "User not found"
            }))
        },
        Err(err) => {
            error!("Failed to fetch user: {:?}", err);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to fetch user",
                "details": err.to_string()
            }))
        }
    }
}