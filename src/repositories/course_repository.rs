use sqlx::{Pool, Postgres};
use uuid::Uuid;
use crate::models::course::{Course, CreateVideoRequest, Enrollment, Video};
use chrono::{ Utc };
use std::error::Error;
use crate::handlers::course_handler::CreateCourseRequest;

#[derive(Clone)]
pub struct CourseRepository {
    pool: Pool<Postgres>
}

impl CourseRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn create_course(&self, request: CreateCourseRequest) -> Result<Course, Box<dyn Error>> {
        let course = sqlx::query_as!(
            Course,
            r#"
            INSERT INTO courses (title, description, price)
            VALUES ($1, $2, $3)
            RETURNING id, title, description, price, created_at, updated_at
            "#,
            request.title,
            request.description,
            request.price
        )
            .fetch_one(&self.pool)
            .await?;

        Ok(course)
    }

    pub async fn create_enrollment(&self, user_id: Uuid, course_id: Uuid) -> Result<Enrollment, Box<dyn Error>> {
        let enrollment = sqlx::query_as!(
            Enrollment,
            r#"
            INSERT INTO enrollments (user_id, course_id)
            VALUES ($1,$2)
            RETURNING id, user_id, course_id, created_at, expires_at
            "#,
            user_id,
            course_id
        )
            .fetch_one(&self.pool)
            .await?;

        Ok(enrollment)
    }

    pub async fn get_all_courses(&self) -> Result<Vec<Course>, Box<dyn Error>> {
        let courses = sqlx::query_as!(
            Course,
            r#"
            SELECT id, title, description, price, created_at, updated_at
            FROM courses
            ORDER BY created_at DESC
            "#
        )
            .fetch_all(&self.pool)
            .await?;

        Ok(courses)
    }

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

    pub async fn get_video(&self, video_id: Uuid) -> Result<Option<Video>, Box<dyn Error>> {
        let video = sqlx::query_as!(
            Video,
            r#"
            SELECT
                *
            FROM videos
            WHERE id = $1
            "#,
            video_id
        )
            .fetch_optional(&self.pool)
            .await?;

        Ok(video)
    }

    pub async fn get_all_videos_for_course(&self, course_id: Uuid) -> Result<Vec<Video>, Box<dyn Error>> {
       let videos_for_course = sqlx::query_as!(
           Video,
           r#"
           SELECT * FROM videos
           WHERE course_id = $1
           "#,
           course_id
       )
           .fetch_all(&self.pool)
           .await?;

        Ok(videos_for_course)
    }

    pub async fn create_video(&self, new_video: CreateVideoRequest) -> Result<Video, Box<dyn Error>> {
        let video = sqlx::query_as!(
            Video,
            r#"
            INSERT INTO videos (course_id, title, upload_id, description, mux_asset_id, mux_playback_id, duration)
            VALUES ($1, $2, $3, NULL, NULL, NULL, NULL)
            RETURNING id, course_id, title, description, upload_id, status, course_value, mux_asset_id, mux_playback_id, duration, created_at, updated_at
            "#,
            new_video.course_id,
            new_video.title,
            new_video.upload_id,
        )
            .fetch_one(&self.pool)
            .await?;

        Ok(video)
    }
}

#[cfg(test)]
mod course_repository_tests {
    use super::*;
    use chrono::{Duration, Utc};
    use sqlx::postgres::PgPoolOptions;
    use dotenv::dotenv;
    use uuid::Uuid;

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
            99
        )
            .execute(pool)
            .await
            .expect("Failed to create test course");

        course_id
    }

    async fn _create_test_video(pool: &Pool<Postgres>, course_id: Uuid) -> Uuid {
        let video_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO videos (id, course_id, title, description, mux_asset_id, mux_playback_id, duration)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            video_id,
            course_id,
            "Test Video",
            Some("Test Video Description"),
            "test_asset_id",
            "test_playback_id",
            120.5,
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
    async fn test_get_video_returns_none_for_nonexistent_video() {
        let pool = setup_test_db().await;
        let repo = CourseRepository::new(pool);

        let video_id = Uuid::new_v4(); // Random UUID that doesn't exist

        let result = repo.get_video(video_id).await;

        assert!(result.is_ok(), "get_video should not error");
        let video = result.unwrap();
        assert!(video.is_none(), "Video should not exist");
    }
}