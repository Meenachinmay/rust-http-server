use actix_web::{web, HttpResponse, Responder, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::auth::extractor::AuthUser;
use crate::models::course::{CreateVideoRequest, VideoPlaybackRequest};
use crate::repositories::course_repository::CourseRepository;
use crate::repositories::user_repository::UserRepository;
use crate::services::mux_service::MuxService;

#[derive(Serialize)]
struct UploadUrlResponse {
    upload_id: String,
    upload_url: String,
}

#[derive(Serialize)]
struct PlaybackTokenResponse {
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadCreateVideoRequest {
    course_id: Uuid,
    title: String
}

pub async fn get_video_token(
    req: web::Json<VideoPlaybackRequest>,
    auth_user: AuthUser,
    course_repo: web::Data<CourseRepository>,
    user_repo: web::Data<UserRepository>,
    mux_service: web::Data<MuxService>,
) -> Result<HttpResponse> {
    let video_id = req.video_id;

    let user = match auth_user.get_user_from_db(&user_repo).await {
        Ok(u) => u,
        Err(_) => return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to retrieve user information"
        }))),
    };

    let video = match course_repo.get_video(video_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Video not found"
        }))),
        Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
    };

    let is_enrolled = match course_repo.is_user_enrolled(user.uid, video.course_id).await {
        Ok(enrolled) => enrolled,
        Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
    };

    if !is_enrolled {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "You are not enrolled in this course"
        })));
    }

    let token = match &video.mux_playback_id {
        Some(id) => mux_service.create_playback_token(id),
        None => return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "No playback Id is provided yet"
        }))),
    };

    let token_string = token?.to_string();
    let response = PlaybackTokenResponse { token: token_string };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn create_upload_url(
    mux_service: web::Data<MuxService>,
    course_repo: web::Data<CourseRepository>,
    payload: web::Json<UploadCreateVideoRequest>
) -> impl Responder {
    match mux_service.create_direct_upload().await {
        Ok(upload_data) => {
            // save the video here with uploadID before returning to user.
            let video_request = CreateVideoRequest {
                course_id: payload.course_id,
                title: payload.title.clone(),
                upload_id: upload_data.id.clone(),
            };

            match course_repo.create_video(video_request).await {
                Ok(_) => {
                    // Return the upload URL and ID
                    HttpResponse::Ok().json(UploadUrlResponse {
                        upload_id: upload_data.id,
                        upload_url: upload_data.url,
                    })
                },
                Err(e) => {
                    eprintln!("error saving video record: {:?}", e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                       "error": "Failed to save video record in DB"
                    }))
                }
            }
        },
        Err(e) => {
            eprintln!("Error creating upload URL: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create upload URL"
            }))
        }
    }
}

#[cfg(test)]
mod video_handler_tests {
    use super::*;
    use actix_web::{ web, http::StatusCode};
    use dotenv::dotenv;
    use sqlx::postgres::{PgPool, PgPoolOptions};
    use uuid::Uuid;
    use crate::models::user::User;
    use crate::auth::extractor::AuthUser;
    use crate::config::mux::MuxConfig;
    use std::fs;

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

    // Helper function to get a test private key for Mux
    fn get_test_private_key() -> String {
        // Try to read from the actual PEM file first
        match fs::read_to_string("mux.pem") {
            Ok(key) => key,
            // Fall back to a test key if file doesn't exist
            Err(_) => r#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAvxW2wuTTK2d0ob5mu/ASJ9vYDc/SXy06QAqRQACABoJxO3JV
Kqh5HZGsn9SjxSqGBdRXCoRG8nhwuuqJaHhJ+SxYWKS+RBt/vwgYC6CJSuVL6EqC
WqCdwSCEMJxGHLM+cG7smCb0Po9xv8tIGR4NpBXzRWL/2/6MZZSGWs5bKGNOxzAJ
COmX9+G3vpGxmZgB1uUMHEecJ+m0NRwZxC8ZnHBgvE8Hu9HjVNg4CE4QJMzqQuR3
6GXtfwp//ER0lfBQS0fLv9OJlTEFgbP7dsp2LUfHiRjqJy0o6kQ8QrrNTImyMRZt
zDQImZkqQCoB7HCN3gcrK8sGqn0GxHN1hq4EkQIDAQABAoIBAQCGIUcfqLJLKCHr
kcV1iPVDPqWnLPlBkNHcCM5De3S+RnFRUjAkHfzvCfXTFGZFI0cFmaJK1Ds9D8XN
h1+K3iqCUbfg7GFNheCu0kX4iant0Z3KDTw4xjbJkbIFdKV/bMvF+XgTjKsOJ5KU
FqmU4HVdKCj41GqvnTQNgUD7XgQVs6WHlLLKbbw0+AyJvnbQY9j4DWbJz/ch5HvK
KUYsuJPvVj3EHGHzTs0DZB2GnNi3l6EOlbYeR7l0ccQFciUfQJLHb4WJ/Pl0FGnI
emeIQqGRukMBK5bZxA7p7tr44Hx7+IZNFyfgOtY+vnA7WXvOeIaWpnSzrOitHPyn
Nw6mjDyBAoGBAOk5s6ljbgvZ7TXy3sr87m9Z5+9NQEDdiJ+mTvJxs7+U/dPP61We
mO3LNKnY5QRgOZlyWWS+x1HVaS39v66Q4UhmrNUbdvT/pITYkTz2Qg/XGIpz/4G2
4cRmY7hWRFWCL5W3NuGZgkqjmxCl8mYqvrGKcgIZ/wkaMILlOz0AHkydAoGBANEz
UT0yHN9vwj8Elv7A4qKvvzPkS+rKtUmZ1BOvCJY0QI//bAFCFgLRGT1BD3D6kfWb
a/7/lDusckgUOXvLrfEPMgcPBkKFvZPTj/iZ7V7HVFgOUJXjL8jW9OsVRUKyLJvq
ODCJIqC8HqjrJXU3mHUWTePPbipQXMFHQJNWBeplAoGAFPeYR1CRQyQIa9fbaDlY
Xc5kUwRlJvO7Tlp9rZrKWKZAQcOCAtXLYT+x+YhOLqEZz/Z2vUqUV6+8yclHnD/0
Pdx33c3SjdZ8aP+NH9zXWQmg4LS1Tp3oTG55W7V3t+bWKt78WO4XJzQsZzSlGZTG
HvEDwrsHKmajZZabG3Wr5y0CgYEAoQc3UEz6S9nAyjdejnFQ8Ccd4vtALdejJ1GA
YGzPj274AYdWGi1hCkFs+3dhqRRgiffufpGT/JVz3GZvkn4zZ8Cx3O/xAXx+mJcA
Zz8uZG4kQMHbquhd9+6hRQhA4fBx5Ejz3xvxcxtuFYR53nMvLfLHFDs3ZtcCXSWk
1pzpGt0CgYB3gzgKGbni2S+H7cN0+Y1rWgGYoYeKj713zkKGwZHNSGQRPPaFzLxt
2SfKJmhMqW7ya3KLYJGJYdHVZDKNdJnTQ+D9c3OZyio5YnA3WVlpQbY7g9oKV6RJ
xlj/JvsGMFJtkCYYC6uZKMQXD5Z5UzQ7aia8g5ppzjRW2hHwDPgyqQ==
-----END RSA PRIVATE KEY-----"#.to_string()
        }
    }

    async fn setup_test_data(pool: &PgPool) -> (Uuid, Uuid, Uuid) {
        // Create a test user
        let user_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO users (uid, email, password)
            VALUES ($1, $2, $3)
            "#,
            user_id,
            format!("test_{}@example.com", user_id),
            "password_hash"
        )
            .execute(pool)
            .await
            .expect("Failed to create test user");

        // Create a test course
        let course_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO courses (id, title, description, price)
            VALUES ($1, $2, $3, $4)
            "#,
            course_id,
            "Test Course",
            Some("Test Description"),
            99
        )
            .execute(pool)
            .await
            .expect("Failed to create test course");

        // Create a test video
        let video_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO videos (id, course_id, title, description, mux_asset_id, mux_playback_id, duration)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            video_id,
            course_id,
            "Test Video",
            Some("Test Video Description"),
            "test_asset_id",
            "test_playback_id",
            120.5,
        )
            .execute(pool)
            .await
            .expect("Failed to create test video");

        (user_id, course_id, video_id)
    }

    #[tokio::test]
    async fn test_get_video_token_succeeds_for_enrolled_user() {
        let pool = setup_test_db().await;
        let (user_id, course_id, video_id) = setup_test_data(&pool).await;

        // Create an enrollment
        sqlx::query!(
            r#"
            INSERT INTO enrollments (id, user_id, course_id)
            VALUES ($1, $2, $3)
            "#,
            Uuid::new_v4(),
            user_id,
            course_id
        )
            .execute(&pool)
            .await
            .expect("Failed to create test enrollment");

        // Create repositories and services
        let user_repo = web::Data::new(UserRepository::new(pool.clone()));
        let course_repo = web::Data::new(CourseRepository::new(pool.clone()));

        // Create a test MuxService with updated config structure
        let mux_config = MuxConfig {
            token_id: "test_token_id".to_string(),
            token_secret: "test_token_secret".to_string(),
            signing_key_id: "test_signing_key_id".to_string(), // Changed from signing_key
            private_key: get_test_private_key(), // Added private_key
            base_url: "https://api.mux.com".to_string(),
        };
        let mux_service = web::Data::new(MuxService::new(mux_config));

        // Create mock authenticated user
        let user = User {
            uid: user_id,
            email: format!("test_{}@example.com", user_id),
            password: "password_hash".to_string(),
        };

        let auth_user = AuthUser {
            email: user.email.clone()
        };

        // Create request payload
        let request = VideoPlaybackRequest {
            video_id,
        };

        // Call the handler directly
        let result = get_video_token(
            web::Json(request),
            auth_user,
            course_repo,
            user_repo,
            mux_service,
        ).await;

        // Verify success response
        assert!(result.is_ok(), "Handler should return a response");
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "Status should be OK");

        // Parse response body
        let body_bytes = actix_web::body::to_bytes(response.into_body()).await.unwrap();
        let json_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert!(json_response.get("token").is_some(), "Response should contain token");
    }

    #[tokio::test]
    async fn test_get_video_token_fails_for_non_enrolled_user() {
        let pool = setup_test_db().await;
        let (user_id, _, video_id) = setup_test_data(&pool).await;

        // No enrollment is created

        // Create repositories and services
        let user_repo = web::Data::new(UserRepository::new(pool.clone()));
        let course_repo = web::Data::new(CourseRepository::new(pool.clone()));

        // Create a test MuxService with updated config structure
        let mux_config = MuxConfig {
            token_id: "test_token_id".to_string(),
            token_secret: "test_token_secret".to_string(),
            signing_key_id: "test_signing_key_id".to_string(),
            private_key: get_test_private_key(), // Added private_key
            base_url: "https://api.mux.com".to_string(),
        };
        let mux_service = web::Data::new(MuxService::new(mux_config));

        // Create mock authenticated user
        let user = User {
            uid: user_id,
            email: format!("test_{}@example.com", user_id),
            password: "password_hash".to_string(),
        };

        let auth_user = AuthUser {
            email: user.email.clone()
        };

        // Create request payload
        let request = VideoPlaybackRequest {
            video_id,
        };

        // Call the handler directly
        let result = get_video_token(
            web::Json(request),
            auth_user,
            course_repo,
            user_repo,
            mux_service,
        ).await;

        // Verify forbidden response
        assert!(result.is_ok(), "Handler should return a response");
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "Status should be FORBIDDEN");

        // Parse response body
        let body_bytes = actix_web::body::to_bytes(response.into_body()).await.unwrap();
        let json_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert!(json_response.get("error").is_some(), "Response should contain error");
        assert_eq!(
            json_response["error"].as_str().unwrap(),
            "You are not enrolled in this course",
            "Error message should indicate not enrolled"
        );
    }

    #[tokio::test]
    async fn test_get_video_token_fails_for_nonexistent_video() {
        let pool = setup_test_db().await;
        let (user_id, course_id, _) = setup_test_data(&pool).await;

        // Create an enrollment
        sqlx::query!(
            r#"
            INSERT INTO enrollments (id, user_id, course_id)
            VALUES ($1, $2, $3)
            "#,
            Uuid::new_v4(),
            user_id,
            course_id
        )
            .execute(&pool)
            .await
            .expect("Failed to create test enrollment");

        // Create repositories and services
        let user_repo = web::Data::new(UserRepository::new(pool.clone()));
        let course_repo = web::Data::new(CourseRepository::new(pool.clone()));

        // Create a test MuxService with updated config structure
        let mux_config = MuxConfig {
            token_id: "test_token_id".to_string(),
            token_secret: "test_token_secret".to_string(),
            signing_key_id: "test_signing_key_id".to_string(), // Changed from signing_key
            private_key: get_test_private_key(), // Added private_key
            base_url: "https://api.mux.com".to_string(),
        };
        let mux_service = web::Data::new(MuxService::new(mux_config));

        // Create mock authenticated user
        let user = User {
            uid: user_id,
            email: format!("test_{}@example.com", user_id),
            password: "password_hash".to_string(),
        };

        let auth_user = AuthUser {
            email: user.email.clone()
        };

        // Create request with non-existent video ID
        let request = VideoPlaybackRequest {
            video_id: Uuid::new_v4(), // A random non-existent video ID
        };

        // Call the handler directly
        let result = get_video_token(
            web::Json(request),
            auth_user,
            course_repo,
            user_repo,
            mux_service,
        ).await;

        // Verify not found response
        assert!(result.is_ok(), "Handler should return a response");
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "Status should be NOT_FOUND");

        // Parse response body
        let body_bytes = actix_web::body::to_bytes(response.into_body()).await.unwrap();
        let json_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert!(json_response.get("error").is_some(), "Response should contain error");
        assert_eq!(
            json_response["error"].as_str().unwrap(),
            "Video not found",
            "Error message should indicate video not found"
        );
    }
}