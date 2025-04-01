use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use crate::models::user::{CreateUserRequest, User};
use sqlx::PgPool;
use uuid::Uuid;

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // Creates a new user with a hashed password
    pub async fn create_user(&self, user: CreateUserRequest) -> Result<User, sqlx::Error> {
        let uid = Uuid::new_v4();

        // Hash the password using Argon2 - a secure password hashing algorithm
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(user.password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        // Insert the user with the hashed password
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (uid, email, password)
            VALUES ($1, $2, $3)
            RETURNING uid, email, password
            "#,
            uid,
            user.email,
            password_hash  // Store the hashed password
        )
            .fetch_one(&self.pool)
            .await?;

        Ok(user)
    }

    pub async fn get_user_by_id(&self, uid: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT uid, email, password
            FROM users
            WHERE uid = $1
            "#,
            uid
        )
            .fetch_optional(&self.pool)
            .await
    }


    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT uid, email, password
            FROM users
            WHERE email = $1
            "#,
            email
        )
            .fetch_optional(&self.pool)
            .await
    }
    pub async fn authenticate_user(&self, email: &str, password: &str) -> Result<bool, AuthError> {
        let user = self.get_user_by_email(email).await
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?
            .ok_or(AuthError::UserNotFound)?;

        let parsed_hash = PasswordHash::new(&user.password)
            .map_err(|e| AuthError::HashError(e.to_string()))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

#[derive(Debug)]
pub enum AuthError {
    DatabaseError(String),
    UserNotFound,
    HashError(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::DatabaseError(e) => write!(f, "Database error: {}", e),
            AuthError::UserNotFound => write!(f, "User not found"),
            AuthError::HashError(e) => write!(f, "Password hash error: {}", e),
        }
    }
}

impl std::error::Error for AuthError {}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use sqlx::postgres::{PgPool, PgPoolOptions};
    use crate::models::user::CreateUserRequest;

    async fn create_test_pool() -> PgPool {
        dotenv().ok();

        let database_url = std::env::var("DATABASE_URL_TEST").expect("DATABASE_URL must be set");

        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to create database pool")
    }

    #[tokio::test]
    async fn test_create_user() {
        // Create a pool
        let pool = create_test_pool().await;

        // Create a repository
        let repo = UserRepository::new(pool);

        // Create a test user request
        let test_email = format!("test_{}@example.com", Uuid::new_v4());
        let user_request = CreateUserRequest {
            email: test_email.clone(),
            password: "TestPassword123!".to_string(),
        };

        // Test creating a user
        let result = repo.create_user(user_request).await;
        assert!(result.is_ok(), "Should create a user successfully");

        // Verify the user data
        let user = result.unwrap();
        assert_eq!(user.email, test_email, "Email should match");
        assert_ne!(user.password, "TestPassword123!", "Password should be hashed");
    }

    #[tokio::test]
    async fn test_get_user_by_id() {
        // Create a pool
        let pool = create_test_pool().await;

        // Create a repository
        let repo = UserRepository::new(pool);

        // Create a test user
        let test_email = format!("test_get_{}@example.com", uuid::Uuid::new_v4());
        let user_request = CreateUserRequest {
            email: test_email.clone(),
            password: "TestPassword123!".to_string(),
        };

        let created_user = repo.create_user(user_request).await.unwrap();

        // Test getting the user by ID
        let result = repo.get_user_by_id(created_user.uid).await;
        assert!(result.is_ok(), "Should get a user successfully");

        // Verify the user data
        let found_user = result.unwrap();
        assert!(found_user.is_some(), "User should be found");
        let found_user = found_user.unwrap();
        assert_eq!(found_user.uid, created_user.uid, "User ID should match");
        assert_eq!(found_user.email, test_email, "Email should match");
    }

    #[tokio::test]
    async fn test_get_user_by_email() {
        let pool = create_test_pool().await;

        let repo = UserRepository::new(pool);

        let test_email = format!("test_email_{}@example.com", uuid::Uuid::new_v4());
        let user_request = CreateUserRequest {
            email: test_email.clone(),
            password: "TestPassword123!".to_string(),
        };

        repo.create_user(user_request).await.unwrap();

        let result = repo.get_user_by_email(&test_email).await;
        assert!(result.is_ok(), "Should get a user by email successfully");

        // Verify the user data
        let found_user = result.unwrap();
        assert!(found_user.is_some(), "User should be found");
        let found_user = found_user.unwrap();
        assert_eq!(found_user.email, test_email, "Email should match");
    }

    #[tokio::test]
    async fn test_authenticate_user() {
        let pool = create_test_pool().await;

        let repo = UserRepository::new(pool);

        let test_email = format!("test_auth_{}@example.com", uuid::Uuid::new_v4());
        let password = "TestPassword123!";
        let user_request = CreateUserRequest {
            email: test_email.clone(),
            password: password.to_string(),
        };

        repo.create_user(user_request).await.unwrap();

        let result = repo.authenticate_user(&test_email, password).await;
        assert!(result.is_ok(), "Authentication should succeed");
        assert!(result.unwrap(), "Authentication should return true for correct password");

        let result = repo.authenticate_user(&test_email, "WrongPassword123!").await;
        assert!(result.is_ok(), "Authentication should not error");
        assert!(!result.unwrap(), "Authentication should return false for incorrect password");
    }
}