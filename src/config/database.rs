use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{info, error};

pub async fn create_pool() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL").unwrap();

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

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;

    #[tokio::test]
    async fn test_create_pool_success() {
        dotenv().ok();

        let result = create_pool().await;
        assert!(result.is_ok(), "Should create a database pool successfully");

        if let Ok(pool) = result {
            let query_result = sqlx::query("SELECT 1").fetch_one(&pool).await;
            assert!(query_result.is_ok(), "Should execute a simple query");
        }
    }

}