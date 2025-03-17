use sqlx::{Pool, Postgres};
use uuid::Uuid;
use crate::models::course::{Course, Video };
use chrono::{ Utc };
use std::error::Error;

pub struct CourseRepository {
    pool: Pool<Postgres>
}

impl CourseRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    // Check if a user is enrolled in a course
    pub async fn is_user_enrolled(&self, user_id: Uuid, course_id: Uuid) -> Result<bool, Box<dyn Error>> {
        let now = Utc::now();

        let result = sqlx::query!(
            r#"
            SELECT id FROM enrollments
            WHERE user_id = $1 AND course_id = $2
            AND (expires_at IS NULL OR expires_at > $3)
            "#,
            user_id,
            course_id,
            now
        )
            .fetch_optional(&self.pool)
            .await?;

        Ok(result.is_some())
    }

    // Get a video by ID
    pub async fn get_video(&self, video_id: Uuid) -> Result<Option<Video>, Box<dyn Error>> {
        let video = sqlx::query_as!(
            Video,
            r#"
            SELECT
                id, course_id, title, description,
                mux_asset_id, mux_playback_id, duration,
                "order", created_at, updated_at
            FROM videos
            WHERE id = $1
            "#,
            video_id
        )
            .fetch_optional(&self.pool)
            .await?;

        Ok(video)
    }

    // Get course for a video
    pub async fn get_video_course(&self, video_id: Uuid) -> Result<Option<Course>, Box<dyn Error>> {
        let course = sqlx::query_as!(
            Course,
            r#"
            SELECT c.id, c.title, c.description, c.price, c.created_at, c.updated_at
            FROM courses c
            JOIN videos v ON v.course_id = c.id
            WHERE v.id = $1
            "#,
            video_id
        )
            .fetch_optional(&self.pool)
            .await?;

        Ok(course)
    }

}

#[cfg(test)]
mod course_repository_tests {
    use super::*;
    use chrono::{Duration, Utc};
    use sqlx::postgres::PgPoolOptions;
    use dotenv::dotenv;
    use uuid::Uuid;
    use bigdecimal::BigDecimal;

    async fn setup_test_db() -> Pool<Postgres> {
        dotenv().ok();

        let database_url = std::env::var("DATABASE_URL_TEST")
            .expect("DATABASE_URL_TEST must be set for tests");

        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to create test pool")
    }

    async fn create_test_course(pool: &Pool<Postgres>) -> Uuid {
        let course_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO courses (id, title, description, price)
            VALUES ($1, $2, $3, $4)
            "#,
            course_id,
            "Test Course",
            Some("Test Description"),
            BigDecimal::from(99)
        )
            .execute(pool)
            .await
            .expect("Failed to create test course");

        course_id
    }

    async fn create_test_video(pool: &Pool<Postgres>, course_id: Uuid) -> Uuid {
        let video_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO videos (id, course_id, title, description, mux_asset_id, mux_playback_id, duration, "order")
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            video_id,
            course_id,
            "Test Video",
            Some("Test Video Description"),
            "test_asset_id",
            "test_playback_id",
            120.5,
            1
        )
            .execute(pool)
            .await
            .expect("Failed to create test video");

        video_id
    }

    async fn create_test_user(pool: &Pool<Postgres>) -> Uuid {
        let user_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO users (uid, email, password)
            VALUES ($1, $2, $3)
            "#,
            user_id,
            format!("test_{}@example.com", user_id),
            "password_hash"
        )
            .execute(pool)
            .await
            .expect("Failed to create test user");

        user_id
    }

    async fn create_test_enrollment(
        pool: &Pool<Postgres>,
        user_id: Uuid,
        course_id: Uuid,
        expired: bool
    ) -> Uuid {
        let enrollment_id = Uuid::new_v4();
        let expires_at = if expired {
            Some(Utc::now() - Duration::hours(1))
        } else {
            Some(Utc::now() + Duration::days(30))
        };

        sqlx::query!(
            r#"
            INSERT INTO enrollments (id, user_id, course_id, expires_at)
            VALUES ($1, $2, $3, $4)
            "#,
            enrollment_id,
            user_id,
            course_id,
            expires_at
        )
            .execute(pool)
            .await
            .expect("Failed to create test enrollment");

        enrollment_id
    }

    #[tokio::test]
    async fn test_is_user_enrolled_returns_true_for_enrolled_user() {
        let pool = setup_test_db().await;
        let repo = CourseRepository::new(pool.clone());

        let user_id = create_test_user(&pool).await;
        let course_id = create_test_course(&pool).await;

        // Create an active enrollment
        create_test_enrollment(&pool, user_id, course_id, false).await;

        let result = repo.is_user_enrolled(user_id, course_id).await;

        assert!(result.is_ok(), "is_user_enrolled should not error");
        let is_enrolled = result.unwrap();
        assert!(is_enrolled, "User should be enrolled in the course");
    }

    #[tokio::test]
    async fn test_is_user_enrolled_returns_false_for_non_enrolled_user() {
        let pool = setup_test_db().await;
        let repo = CourseRepository::new(pool.clone());

        let user_id = create_test_user(&pool).await;
        let course_id = create_test_course(&pool).await;

        // Don't create any enrollment

        let result = repo.is_user_enrolled(user_id, course_id).await;

        assert!(result.is_ok(), "is_user_enrolled should not error");
        let is_enrolled = result.unwrap();
        assert!(!is_enrolled, "User should not be enrolled in the course");
    }

    #[tokio::test]
    async fn test_is_user_enrolled_returns_false_for_expired_enrollment() {
        let pool = setup_test_db().await;
        let repo = CourseRepository::new(pool.clone());

        let user_id = create_test_user(&pool).await;
        let course_id = create_test_course(&pool).await;

        // Create an expired enrollment
        create_test_enrollment(&pool, user_id, course_id, true).await;

        let result = repo.is_user_enrolled(user_id, course_id).await;

        assert!(result.is_ok(), "is_user_enrolled should not error");
        let is_enrolled = result.unwrap();
        assert!(!is_enrolled, "User should not be enrolled due to expired enrollment");
    }

    #[tokio::test]
    async fn test_get_video_returns_video_when_exists() {
        let pool = setup_test_db().await;
        let repo = CourseRepository::new(pool.clone());

        let course_id = create_test_course(&pool).await;
        let video_id = create_test_video(&pool, course_id).await;

        let result = repo.get_video(video_id).await;

        assert!(result.is_ok(), "get_video should not error");
        let video = result.unwrap();
        assert!(video.is_some(), "Video should exist");

        let video = video.unwrap();
        assert_eq!(video.id, video_id, "Video ID should match");
        assert_eq!(video.course_id, course_id, "Course ID should match");
        assert_eq!(video.mux_asset_id, "test_asset_id", "Mux asset ID should match");
        assert_eq!(video.mux_playback_id, "test_playback_id", "Mux playback ID should match");
        assert_eq!(video.duration, 120.5, "Video duration should match");
        assert_eq!(video.order, 1, "Video order should match");
    }

    #[tokio::test]
    async fn test_get_video_returns_none_for_nonexistent_video() {
        let pool = setup_test_db().await;
        let repo = CourseRepository::new(pool);

        let video_id = Uuid::new_v4(); // Random UUID that doesn't exist

        let result = repo.get_video(video_id).await;

        assert!(result.is_ok(), "get_video should not error");
        let video = result.unwrap();
        assert!(video.is_none(), "Video should not exist");
    }

    #[tokio::test]
    async fn test_get_video_course_returns_course_when_exists() {
        let pool = setup_test_db().await;
        let repo = CourseRepository::new(pool.clone());

        let course_id = create_test_course(&pool).await;
        let video_id = create_test_video(&pool, course_id).await;

        let result = repo.get_video_course(video_id).await;

        assert!(result.is_ok(), "get_video_course should not error");
        let course = result.unwrap();
        assert!(course.is_some(), "Course should exist");

        let course = course.unwrap();
        assert_eq!(course.id, course_id, "Course ID should match");
        assert_eq!(course.title, "Test Course", "Course title should match");
    }
}