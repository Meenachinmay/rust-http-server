use actix_web::{web, HttpResponse, Responder};
use uuid::Uuid;
use crate::models::user::CreateUserRequest;
use crate::repositories::user_repository::UserRepository;

pub async fn create_user(
    repo: web::Data<UserRepository>,
    user: web::Json<CreateUserRequest>,
) -> impl Responder {
    match repo.create_user(user.into_inner()).await {
        Ok(created_user) => HttpResponse::Created().json(created_user),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn get_user(
    repo: web::Data<UserRepository>,
    id: web::Path<Uuid>,
) -> impl Responder {
    match repo.get_user_by_id(id.into_inner()).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}