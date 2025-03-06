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

    // New method: Authenticate a user by verifying their password
    pub async fn authenticate_user(&self, email: &str, password: &str) -> Result<bool, AuthError> {
        // First, retrieve the user by email
        let user = self.get_user_by_email(email).await
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?
            .ok_or(AuthError::UserNotFound)?;

        // Verify the password using Argon2
        let parsed_hash = PasswordHash::new(&user.password)
            .map_err(|e| AuthError::HashError(e.to_string()))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

// Custom error type for authentication-related errors
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