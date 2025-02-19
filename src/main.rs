mod config;
mod handlers;
mod models;
mod repositories;
mod auth;
mod communication;

use actix_web::{web, App, HttpServer, middleware::Logger};
use auth::middleware::AuthMiddleware;
use handlers::{
    user_handler::{create_user, get_user},
    auth_handler::{signin, signup, set_password}
};
use repositories::user_repository::UserRepository;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use dotenv::dotenv;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    // Load .env file before any environment variables are accessed
    if let Err(e) = dotenv() {
        eprintln!("Failed to load .env file: {}", e);
        // Continue execution as environment variables might be set through other means
    }

    // Verify that required environment variables are present
    let required_vars = ["SENDGRID_API_KEY", "FRONTEND_URL", "SENDER_EMAIL"];
    for var in required_vars.iter() {
        if env::var(var).is_err() {
            eprintln!("Required environment variable '{}' is not set", var);
        }
    }

    // Initialize better logging
    FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_line_number(true)
        .with_file(true)
        .with_thread_ids(true)
        .with_target(false)
        .init();

    info!("Starting server...");

    // Create database pool
    let pool = config::database::create_pool()
        .await
        .expect("Failed to create pool");

    info!("Database pool created successfully");

    // Create user repository
    let user_repository = web::Data::new(UserRepository::new(pool));

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default()) // Add logging middleware
            .wrap(AuthMiddleware) // Let's add auth middleware
            .wrap(Logger::new("%a %r %s %b %{Referer}i %{User-Agent}i %T")) // Detailed logging
            .app_data(user_repository.clone())
            .route("/signup", web::post().to(signup))
            .route("/signin", web::post().to(signin))
            .route("/setpassword", web::post().to(set_password))
            .route("/users", web::post().to(create_user))
            .route("/users/{id}", web::get().to(get_user))
    }).workers(28)
        .bind("127.0.0.1:8080")?
        .run()
        .await
}