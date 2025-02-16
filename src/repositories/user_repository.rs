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

    pub async fn create_user(&self, user: CreateUserRequest) -> Result<User, sqlx::Error> {
        let uid = Uuid::new_v4();

        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (uid, email, password)
            VALUES ($1, $2, $3)
            RETURNING uid, email, password
            "#,
            uid,
            user.email,
            user.password // Remember to hash passwords in production!
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
}