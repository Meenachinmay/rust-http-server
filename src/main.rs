mod config;
mod handlers;
mod models;
mod repositories;

use actix_web::{web, App, HttpServer, middleware::Logger, middleware::ErrorHandlers};
use handlers::user_handler::{create_user, get_user};
use repositories::user_repository::UserRepository;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
            .wrap(Logger::new("%a %r %s %b %{Referer}i %{User-Agent}i %T")) // Detailed logging
            .app_data(user_repository.clone())
            .route("/users", web::post().to(create_user))
            .route("/users/{id}", web::get().to(get_user))
    }).workers(28)
        .bind("127.0.0.1:8080")?
        .run()
        .await
}