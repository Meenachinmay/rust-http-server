mod config;
mod services;
mod handlers;
mod models;
mod repositories;
mod auth;
mod communication;

use actix_web::{middleware::Logger, web, App, HttpServer };
use auth::middleware::AuthMiddleware;
use handlers::{
    auth_handler::{set_password, signin, signup},
    user_handler::get_user,
    video_handler::{
        get_video_token,
        list_videos,
        create_upload_url,
    },
    course_handler::{
        create_course,
        get_courses,
        get_enrollment
    }
};
use repositories::{
    course_repository::CourseRepository,
    user_repository::UserRepository,
};

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use dotenv::dotenv;
use std::env;
use actix_cors::Cors;
use migrations::Migrator;
use services::mux_service::MuxService;
use services::course_service::CourseService;
use config::mux::MuxConfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    if let Err(e) = dotenv() {
        eprintln!("Failed to load .env file: {}", e);
    }

    let required_vars = [
        "SENDGRID_API_KEY",
        "FRONTEND_URL",
        "SENDER_EMAIL",
        "DATABASE_URL",
        "DATABASE_URL_TEST",
        "MUX_TOKEN_ID",
        "MUX_TOKEN_SECRET",
        "MUX_SIGNING_KEY_ID"];
    for var in required_vars.iter() {
        if env::var(var).is_err() {
            eprintln!("Required environment variable '{}' is not set", var);
        }
    }

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    info!("Running database migrations from external repository...");
    let migrator = Migrator::new(&database_url)
        .await
        .expect("Failed to create migrator");

    if let Err(e) = migrator.run_migrations().await {
        eprintln!("Failed to run migrations: {}", e);
    } else {
        info!("Database migrations completed successfully");
    }

    FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_line_number(true)
        .with_file(true)
        .with_thread_ids(true)
        .with_target(false)
        .init();

    let pool = config::database::create_pool()
        .await
        .expect("Failed to create pool");
    info!("Database pool created successfully");

    let user_repository = web::Data::new(UserRepository::new(pool.clone()));
    let course_repository = web::Data::new(CourseRepository::new(pool.clone()));

    info!("Starting server...");
    HttpServer::new(move || {

        // create mux service
        let mux_config = MuxConfig::from_env()
            .expect("Failed to load Mux configuration");
        let mux_service = web::Data::new(MuxService::new(mux_config));
        let course_service = web::Data::new(CourseService::new(course_repository.get_ref().clone()));

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
            .app_data(course_repository.clone())
            .app_data(mux_service)
            .app_data(course_service.clone())
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
                    .route("/videos/token", web::post().to(get_video_token))
                    .route("/videos", web::get().to(list_videos))
                    .route("/mux/upload-url", web::post().to(create_upload_url))
                    .route("/courses", web::post().to(create_course))
                    .route("/courses", web::get().to(get_courses))
                    .route("/courses/enrollment", web::get().to(get_enrollment))
            )
    })
        .workers(28)
        .bind("0.0.0.0:8080")?
        .run()
        .await
}