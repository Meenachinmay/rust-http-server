use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{info, error};

pub async fn create_pool() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://rust_1_user:XEO0mscxYhsykTLSksdlYGUSFlCAD7qK@dpg-cv2q52l6l47c73821js0-a.oregon-postgres.render.com/rust_1".to_string());

    info!("Creating database connection pool...");

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(3))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&database_url)
        .await?;

    // Verify the connection
    match sqlx::query("SELECT 1").fetch_one(&pool).await {
        Ok(_) => info!("Database connection successful"),
        Err(e) => {
            error!("Failed to verify database connection: {}", e);
            return Err(e);
        }
    }

    Ok(pool)
}