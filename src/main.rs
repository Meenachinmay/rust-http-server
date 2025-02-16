mod error;

mod config;
mod handlers;
mod repositories;
mod models;

use actix_web::{web, App, HttpServer};
use handlers::user_handler::{create_user, get_user};
use repositories::user_repository::UserRepository;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create database pool
    let pool = config::database::create_pool()
        .await
        .expect("Failed to create pool");

    // Create user repository
    let user_repository = web::Data::new(UserRepository::new(pool));

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(user_repository.clone())
            .route("/users", web::post().to(create_user))
            .route("/users/{id}", web::get().to(get_user))
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
