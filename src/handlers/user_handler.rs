use actix_web::{web, HttpResponse, Responder};
use uuid::Uuid;
use crate::repositories::user_repository::UserRepository;
use tracing::{error, info, instrument};
use serde_json::json;  // Add this import for json! macro

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