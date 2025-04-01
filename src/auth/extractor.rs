use actix_web::{dev::Payload, Error, FromRequest, HttpRequest, error::ErrorUnauthorized, HttpMessage};
use futures::future::{ready, Ready};
use crate::repositories::user_repository::UserRepository;
use crate::models::user::User;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub email: String,
}

impl FromRequest for AuthUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        // Extract user email from request extensions
        if let Some(user_email) = req.extensions().get::<String>() {
            // Found a user email, create and return the AuthUser
            ready(Ok(AuthUser {
                email: user_email.clone(),
            }))
        } else {
            // This shouldn't happen if your AuthMiddleware is working correctly,
            // but handle it just in case
            ready(Err(ErrorUnauthorized("User not authenticated")))
        }
    }
}

impl AuthUser {
    pub async fn get_user_from_db(&self, user_repo: &UserRepository) -> Result<User, Error> {
        user_repo.get_user_by_email(&self.email)
            .await
            .map_err(|_| ErrorUnauthorized("Failed to retrieve user"))
            .and_then(|maybe_user| {
                maybe_user.ok_or_else(|| ErrorUnauthorized("User not found"))
            })
    }
}