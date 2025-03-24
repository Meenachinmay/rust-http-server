use std::error::Error;
use crate::models::course::Course;
use crate::handlers::course_handler::CreateCourseRequest;
use crate::repositories::course_repository::CourseRepository;
use uuid::Uuid;

pub struct CourseService {
    repo: CourseRepository
}

impl CourseService {
    pub fn new(repo: CourseRepository) -> Self {
        Self { repo }
    }

    pub async fn create_course(&self, request: CreateCourseRequest) -> Result<Course, Box<dyn Error>> {
        self.repo.create_course(request).await
    }

    pub async fn get_all_courses(&self) -> Result<Vec<Course>, Box<dyn Error>> {
        self.repo.get_all_courses().await
    }

    pub async fn is_user_enrolled(&self, user_id: Uuid, course_id: Uuid) -> Result<bool, Box<dyn Error>> {
        self.repo.is_user_enrolled(user_id, course_id).await
    }
}