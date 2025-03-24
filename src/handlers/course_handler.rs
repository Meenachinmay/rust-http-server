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