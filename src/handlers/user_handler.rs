use actix_web::{web, HttpResponse, Responder};
use uuid::Uuid;
use crate::repositories::user_repository::UserRepository;

use tracing::{error, info, instrument};
use serde_json::json;

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
            }))
        }
    }
}

pub async fn get_user_by_email(
    repo: web::Data<UserRepository>,
    email: web::Path<String>,
) -> impl Responder {
    info!("Attempting to fetch user with id: {}", email);

    match repo.get_user_by_email(&email.into_inner()).await {
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
                    "error": "Internal server error",
                }))
        }
    }
}

#[cfg(test)]
mod user_handler_tests {
    use actix_web::{test, web, App, http::StatusCode};
    use dotenv::dotenv;
    use sqlx::postgres::{PgPool, PgPoolOptions};
    use uuid::Uuid;

    use crate::handlers::user_handler::{get_user, get_user_by_email};
    use crate::repositories::user_repository::UserRepository;
    use crate::models::user::{User, CreateUserRequest};

    async fn setup_test_db() -> PgPool {
        dotenv().ok();

        let database_url = std::env::var("DATABASE_URL_TEST")
            .expect("DATABASE_URL_TEST must be set for tests");

        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to create test pool")
    }

    #[tokio::test]
    async fn test_get_user_by_id_success() {
        let pool = setup_test_db().await;
        let user_repo = web::Data::new(UserRepository::new(pool.clone()));

        let test_email = format!("test_user_{}@example.com", Uuid::new_v4());
        let user = user_repo.create_user(CreateUserRequest {
            email: test_email.clone(),
            password: "TestPassword123".to_string(),
        }).await.expect("Failed to create test user");

        let app = test::init_service(
            App::new()
                .app_data(user_repo.clone())
                .route("/users/{id}", web::get().to(get_user))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/users/{}", user.uid))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK, "Should return 200 OK status");

        let body = test::read_body(resp).await;
        let returned_user: User = serde_json::from_slice(&body).unwrap();
        assert_eq!(returned_user.uid, user.uid, "Should return the correct user ID");
        assert_eq!(returned_user.email, user.email, "Should return the correct user email");
    }

    #[tokio::test]
    async fn test_get_user_by_id_not_found() {
        let pool = setup_test_db().await;
        let user_repo = web::Data::new(UserRepository::new(pool.clone()));

        let non_existent_id = Uuid::new_v4();

        let app = test::init_service(
            App::new()
                .app_data(user_repo.clone())
                .route("/users/{id}", web::get().to(get_user))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/users/{}", non_existent_id))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "Should return 404 Not Found status");

        let body = test::read_body(resp).await;
        let error: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(error["error"], "User not found", "Error should indicate user not found");
    }

    #[tokio::test]
    async fn test_get_user_by_email_success() {
        let pool = setup_test_db().await;
        let user_repo = web::Data::new(UserRepository::new(pool.clone()));

        let test_email = format!("test_email_user_{}@example.com", Uuid::new_v4());
        let user = user_repo.create_user(CreateUserRequest {
            email: test_email.clone(),
            password: "TestPassword123".to_string(),
        }).await.expect("Failed to create test user");

        let app = test::init_service(
            App::new()
                .app_data(user_repo.clone())
                .route("/users/email/{email}", web::get().to(get_user_by_email))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/users/email/{}", test_email))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK, "Should return 200 OK status");

        let body = test::read_body(resp).await;
        let returned_user: User = serde_json::from_slice(&body).unwrap();
        assert_eq!(returned_user.uid, user.uid, "Should return the correct user ID");
        assert_eq!(returned_user.email, user.email, "Should return the correct user email");
    }

    #[tokio::test]
    async fn test_get_user_by_email_not_found() {
        let pool = setup_test_db().await;
        let user_repo = web::Data::new(UserRepository::new(pool.clone()));

        let non_existent_email = format!("non_existent_{}@example.com", Uuid::new_v4());

        let app = test::init_service(
            App::new()
                .app_data(user_repo.clone())
                .route("/users/email/{email}", web::get().to(get_user_by_email))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/users/email/{}", non_existent_email))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "Should return 404 Not Found status");

        let body = test::read_body(resp).await;
        let error: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(error["error"], "User not found", "Error should indicate user not found");
    }
}