use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use crate::repositories::user_repository::UserRepository;
use crate::services::course_service::CourseService;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCourseRequest {
    pub title: String,
    pub description: Option<String>,
    pub price: i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetEnrollmentRequest {
    pub course_id: Uuid
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnrollmentQueryParams {
    pub course_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetCourseVideosRequest {
    pub course_id: Uuid
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnrollmentResponse {
    pub user_id: Uuid,
    pub course_id: Uuid,
}

pub async fn create_course(
    course_service: web::Data<CourseService>,
    payload: web::Json<CreateCourseRequest>,
) -> impl Responder {
    match course_service.create_course(payload.into_inner()).await {
        Ok(course) => HttpResponse::Created().json(course),
        Err(e) => {
            eprintln!("Error creating course: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create course"
            }))
        }
    }
}

pub async fn get_courses(
    course_service: web::Data<CourseService>,
) -> impl Responder {
    match course_service.get_all_courses().await {
        Ok(courses) => HttpResponse::Ok().json(courses),
        Err(e) => {
            eprintln!("Error fetching courses: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch courses"
            }))
        }
    }
}

pub async fn get_enrollment(
    req: HttpRequest,
    query: web::Query<GetEnrollmentRequest>,
    course_service: web::Data<CourseService>,
    user_repository: web::Data<UserRepository>
) -> impl Responder {
    let user_email = match req.extensions().get::<String>() {
        Some(email) => email.clone(),
        None => return HttpResponse::Unauthorized().json(json!({
            "error": "USER NOT FOUND"
        }))
    };

    let user = match user_repository.get_user_by_email(&user_email).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().json(json!({
            "error": "User not found"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(json!({
            "error": "Internal server error"
        }))
    };

    match course_service.is_user_enrolled(user.uid, query.course_id).await {
       Ok(is_enrolled) => HttpResponse::Ok().json(json!({
           "enrolled": is_enrolled
       })),
        Err(e) => {
            eprintln!("Error checking enrollment: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Internal server error"
            }))
        }
    }
}

pub async fn enroll_user(
    req: HttpRequest,
    query: web::Query<EnrollmentQueryParams>,
    course_service: web::Data<CourseService>,
    user_repository: web::Data<UserRepository>
) -> impl Responder {
    let user_email = match req.extensions().get::<String>() {
        Some(email) => email.clone(),
        None => return HttpResponse::Unauthorized().json(json!({
            "error": "UNAUTHORIZED"
        }))
    };

    let user = match user_repository.get_user_by_email(&user_email).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::Unauthorized().json(json!({
            "error": "UNAUTHORIZED"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(json!({
            "error": "Internal server error"
        }))
    };

    match course_service.enroll_user(user.uid, query.course_id).await {
        Ok(enrollment) => {
            let response = EnrollmentResponse {
                user_id: enrollment.user_id,
                course_id: enrollment.course_id
            };
            HttpResponse::Created().json(response)
        },
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("already enrolled") {
                HttpResponse::BadRequest().json(json!({
                    "error": error_msg
                }))
            } else {
                eprintln!("Error enrolling user: {:?}", e);
                HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to enroll user"
                }))
            }
        }
    }
}

pub async fn get_course_videos(
    query: web::Query<GetCourseVideosRequest>,
    course_service: web::Data<CourseService>,
) -> impl Responder {
    match course_service.get_videos_for_course(query.course_id).await {
        Ok(videos) => HttpResponse::Ok().json(videos),
        Err(e) => {
            eprintln!("Error getting video videos: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get video videos"
            }))
        }
    }
}

#[cfg(test)]
mod course_handler_tests {
    use actix_web::{test, web, App, http::StatusCode, HttpMessage};
    use dotenv::dotenv;
    use sqlx::postgres::{PgPool, PgPoolOptions};
    use uuid::Uuid;
    use crate::handlers::course_handler::{create_course, enroll_user, get_course_videos, get_courses, CreateCourseRequest, EnrollmentResponse};
    use crate::services::course_service::CourseService;
    use crate::models::course::{Course, Video};
    use crate::models::user::CreateUserRequest;
    use crate::repositories::course_repository::CourseRepository;
    use crate::repositories::user_repository::UserRepository;

    async fn setup_test_db() -> PgPool {
        dotenv().ok();

        let database_url = std::env::var("DATABASE_URL_TEST")
            .expect("DATABASE_URL_TEST is not set");

        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to create test pool")
    }

    #[tokio::test]
    async fn test_create_course_success() {
        let pool = setup_test_db().await;
        let course_service = web::Data::new(CourseService::new(CourseRepository::new(pool.clone())));

        let app = test::init_service(
            App::new()
                .app_data(course_service.clone())
                .route("/api/courses", web::post().to(create_course))
        ).await;

        let course_request = CreateCourseRequest {
            title: "Test course".to_string(),
            description: Some("Test description".to_string()),
            price: 100
        };

        let req = test::TestRequest::post()
            .uri("/api/courses")
            .set_json(&course_request)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::CREATED, "Should return 201 create status");

        let body = test::read_body(resp).await;
        let course: Course = serde_json::from_slice(&body).unwrap();
        assert_eq!(course.title, course_request.title, "Title does not match");
        assert_eq!(course.description, course_request.description, "Description does not match");
        assert_eq!(course.price, course_request.price, "Price does not match");
    }

    #[tokio::test]
    async fn test_get_course_success() {
        let pool = setup_test_db().await;
        let course_service = web::Data::new(CourseService::new(CourseRepository::new(pool.clone())));

        let course_request = CreateCourseRequest {
            title: "Test course".to_string(),
            description: Some("Test description".to_string()),
            price: 100
        };
        course_service.create_course(course_request).await.expect("Failed to create course");

        let app = test::init_service(
            App::new()
                .app_data(course_service.clone())
                .route("/api/courses", web::get().to(get_courses))
        ).await;

        let req = test::TestRequest::get().uri("/api/courses").to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK, "Should return 200");

        let body = test::read_body(resp).await;
        let courses: Vec<Course> = serde_json::from_slice(&body).unwrap();
        assert!(!courses.is_empty(), "Should return at least one course");
    }

    #[tokio::test]
    async fn test_enroll_user_success() {
        let pool = setup_test_db().await;

        let course_repo = CourseRepository::new(pool.clone());
        let user_repo = web::Data::new(UserRepository::new(pool.clone()));
        let course_service = web::Data::new(CourseService::new(course_repo.clone()));

        let course_request = CreateCourseRequest {
            title: "Enrollment Test course".to_string(),
            description: Some("Test description".to_string()),
            price: 100
        };
        let course = course_service.create_course(course_request).await.expect("Failed to create course");

        let test_email = format!("test_user_{}@example.com", Uuid::new_v4());
        let user = user_repo.create_user(CreateUserRequest{
            email: test_email.clone(),
            password: "password123".to_string(),
        }).await.expect("Failed to create user");

        let app = test::init_service(
            App::new()
                .app_data(course_service.clone())
                .app_data(user_repo.clone())
                .route("/api/courses/enrollment", web::post().to(enroll_user))
        ).await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/courses/enrollment?course_id={}", course.id))
            .insert_header(("content-type", "application/json"))
            .set_payload("{}")
            .to_request();

        req.extensions_mut().insert(test_email);

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::CREATED, "Should return 201 created status");

        let is_enrolled = course_service.is_user_enrolled(user.uid, course.id).await.unwrap();
        assert!(is_enrolled,  "User should be enrolled in the course after enrollment");

        let body = test::read_body(resp).await;
        let enrollment_response: EnrollmentResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(enrollment_response.course_id, course.id);
        assert_eq!(enrollment_response.user_id, user.uid)
    }

    #[tokio::test]
    async fn test_get_course_videos_success() {
        let pool = setup_test_db().await;

        let course_repo = CourseRepository::new(pool.clone());
        let course_service = web::Data::new(CourseService::new(course_repo.clone()));

        let course_request = CreateCourseRequest {
            title: "Video Test Course".to_string(),
            description: Some("Course with test videos".to_string()),
            price: 100
        };

        let course = course_service.create_course(course_request).await.expect("Failed to create course");

        let videos_ids = create_test_videos_for_course(&pool, course.id).await;

        let app = test::init_service(
            App::new()
                .app_data(course_service.clone())
                .route("/api/courses/videos", web::get().to(get_course_videos))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/courses/videos?course_id={}", course.id))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK, "Should return 200 OK status");

        let body = test::read_body(resp).await;
        let videos: Vec<Video> = serde_json::from_slice(&body).unwrap();

        assert_eq!(videos.len(), videos_ids.len(), "Should return all videos for the course");

        for video in videos {
            assert_eq!(video.course_id, course.id, "Video should belong to the requests");
        }
    }

    async fn create_test_videos_for_course(pool: &PgPool, course_id: Uuid) -> Vec<Uuid> {
        let mut video_ids = Vec::new();

        for i in 1..=2 {
            let video_id = Uuid::new_v4();
            sqlx::query!(
            r#"
            INSERT INTO videos (id, course_id, title, description, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            "#,
            video_id,
            course_id,
            format!("Test Video {}", i),
            Some(format!("Description for test video {}", i))
        )
                .execute(pool)
                .await
                .expect("Failed to insert test video");

            video_ids.push(video_id);
        }

        video_ids
    }
}