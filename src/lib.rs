// src/lib.rs
pub mod config;
pub mod handlers;
pub mod models;
pub mod repositories;
pub mod auth;
pub mod communication;

// Re-export specific items that tests will need
pub use config::database;
pub use models::user;
pub use repositories::user_repository;
pub use handlers::{user_handler, auth_handler};
pub use auth::middleware;