mod config;
mod handlers;
mod models;
mod repositories;
mod auth;
mod communication;

use actix_web::{web, App, HttpServer, middleware::Logger};
use auth::middleware::AuthMiddleware;
use handlers::{
    user_handler::{get_user},
    auth_handler::{signin, signup, set_password}
};
use repositories::user_repository::UserRepository;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use dotenv::dotenv;
use std::env;
use actix_cors::Cors;
use migrations::Migrator;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    // Load .env file before any environment variables are accessed
    if let Err(e) = dotenv() {
        eprintln!("Failed to load .env file: {}", e);
        // Continue execution as environment variables might be set through other means
    }

    // Verify that required environment variables are present
    let required_vars = ["SENDGRID_API_KEY", "FRONTEND_URL", "SENDER_EMAIL", "DATABASE_URL"];
    for var in required_vars.iter() {
        if env::var(var).is_err() {
            eprintln!("Required environment variable '{}' is not set", var);
        }
    }

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    // Initialize and run migrations from external repository
    info!("Running database migrations from external repository...");
    let migrator = Migrator::new(&database_url)
        .await
        .expect("Failed to create migrator");

    if let Err(e) = migrator.run_migrations().await {
        eprintln!("Failed to run migrations: {}", e);
    } else {
        info!("Database migrations completed successfully");
    }

    // let frontend_url = env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

    // Initialize better logging
    FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_line_number(true)
        .with_file(true)
        .with_thread_ids(true)
        .with_target(false)
        .init();

    // Create database pool
    let pool = config::database::create_pool()
        .await
        .expect("Failed to create pool");
    info!("Database pool created successfully");

    // Create user repository
    let user_repository = web::Data::new(UserRepository::new(pool));

    // Start HTTP server
    info!("Starting server...");
    HttpServer::new(move || {
        // Create a more permissive CORS configuration that explicitly includes your frontend URL
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec!["Authorization", "Content-Type"])
            .supports_credentials()
            .max_age(3600);

        // Create the main App
        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .wrap(Logger::new("%a %r %s %b %{Referer}i %{User-Agent}i %T"))
            .app_data(user_repository.clone())
            .service(
                web::scope("api/auth")
                    .route("/signup", web::post().to(signup))
                    .route("/signin", web::post().to(signin))
            )
            .service(
                web::scope("api")
                    .wrap(AuthMiddleware)
                    .route("/users/{id}", web::get().to(get_user))
                    .route("/setpassword", web::post().to(set_password))
            )
    })
        .workers(28)
        .bind("0.0.0.0:8080")?
        .run()
        .await
}