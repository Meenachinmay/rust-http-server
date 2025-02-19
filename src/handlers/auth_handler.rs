use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info};

use crate::auth::jwt::{generate_token, validate_token};
use crate::communication::email::send_verification_email;
use crate::models::user::CreateUserRequest;
use crate::repositories::user_repository::{AuthError, UserRepository};

#[derive(Deserialize)]
pub struct SignupRequest {
   email: String,
}

#[derive(Deserialize)]
pub struct SigninRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct SetpasswordRequest {
    password: String,
}

// Extract token from Authorization header using a helper function
fn extract_token_from_header(req: &HttpRequest) -> Result<String, &'static str> {
    // Get the Authorization header value
    let auth_header = req.headers()
        .get("Authorization")
        .ok_or("Missing Authorization header")?
        .to_str()
        .map_err(|_| "Invalid Authorization header")?;

    // Check if it starts with "Bearer " and extract the token
    if auth_header.starts_with("Bearer ") {
        Ok(auth_header[7..].to_string())  // Skip "Bearer " prefix
    } else {
        Err("Invalid Authorization header format")
    }
}

pub async fn signup(signup_req: web::Json<SignupRequest>, repo: web::Data<UserRepository>) -> impl Responder {
    info!("Signup request for email: {}", signup_req.email);

    // Check if user already exists
    match repo.get_user_by_email(&signup_req.email).await {
        Ok(Some(_)) => {
            HttpResponse::BadRequest().json(json!({
                "error": "User with this email already exists"
            }));
        }
        Ok(None) => (),
        Err(e) => {
            error!("Database error: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Internal server error"
            }));
        }
    }

    // Generate verification token
    match generate_token(signup_req.email.clone()) {
        Ok(token) => {
            // Send verification email
            match send_verification_email(&signup_req.email, &token).await {
                Ok(_) => HttpResponse::Ok().json(json!({
                    "message": "Verification email sent successfully"
                })),
                Err(e) => {
                    error!("Failed to send verification email: {:?}", e);
                    HttpResponse::InternalServerError().json(json!({
                        "error": "Failed to send verification email"
                    }))
                }
            }
        }
        Err(e) => {
            error!("Token generation failed: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to generate verification token"
            }))
        }
    }

}

pub async fn set_password(
    req: HttpRequest,  // Add HttpRequest parameter to access headers
    password_req: web::Json<SetpasswordRequest>,
    repo: web::Data<UserRepository>,
) -> impl Responder {
    info!("Processing set password request");

    // First, try to extract the token from the Authorization header
    let token = match extract_token_from_header(&req) {
        Ok(token) => token,
        Err(error_msg) => {
            error!("Authorization header error: {}", error_msg);
            return HttpResponse::BadRequest().json(json!({
                "error": error_msg
            }));
        }
    };

    // Validate the extracted token
    match validate_token(token) {
        Ok(claims) => {
            // Create new user with email from token claims and password from request
            let create_user_req = CreateUserRequest {
                email: claims.sub,  // Email is stored in the 'sub' claim
                password: password_req.password.clone(),
            };

            // Attempt to create the user in the database
            match repo.create_user(create_user_req).await {
                Ok(created_user) => {
                    // Generate a new authentication token for the created user
                    match generate_token(created_user.email) {
                        Ok(auth_token) => {
                            info!("User created successfully");
                            HttpResponse::Ok().json(json!({
                                "message": "User created successfully",
                                "token": auth_token,
                                "expires_in": 3600  // Token expiration in seconds
                            }))
                        },
                        Err(e) => {
                            error!("Failed to generate authentication token: {:?}", e);
                            HttpResponse::InternalServerError().json(json!({
                                "error": "Failed to generate authentication token",
                                "details": e.to_string()
                            }))
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to create user: {:?}", e);
                    HttpResponse::InternalServerError().json(json!({
                        "error": "Failed to create user",
                        "details": e.to_string()
                    }))
                }
            }
        },
        Err(e) => {
            error!("Token validation failed: {:?}", e);
            HttpResponse::Unauthorized().json(json!({
                "error": "Invalid or expired token",
                "details": e.to_string()
            }))
        }
    }
}

// Handler for signin
pub async fn signin(
    signin_req: web::Json<SigninRequest>,
    repo: web::Data<UserRepository>,
) -> impl Responder {
    info!("Signin request for email: {}", signin_req.email);

    match repo.authenticate_user(&signin_req.email, &signin_req.password).await {
        Ok(true) => {
            match generate_token(signin_req.email.clone()) {
                Ok(token) => HttpResponse::Ok().json(json!({
                    "token": token,
                    "expires_in": 3600
                })),
                Err(e) => {
                    error!("Token generation failed: {:?}", e);
                    HttpResponse::InternalServerError().json(json!({
                        "error": "Failed to generate token"
                    }))
                }
            }
        }
        Ok(false) => HttpResponse::Unauthorized().json(json!({
            "error": "Invalid credentials"
        })),
        Err(e) => {
            error!("Authentication error: {:?}", e);
            // match e {
            //     AuthError::UserNotFound => HttpResponse::NotFound(),
            //     _ => HttpResponse::InternalServerError(),
            // }
            HttpResponse::InternalServerError().json(json!({
                "error": "Authentication failed"
            }))
        }
    }
}