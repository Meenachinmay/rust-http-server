use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse, http::header,
    body::EitherBody,
};
use futures::future::{ready, LocalBoxFuture, Ready};
use tracing::{error, info};
use serde_json::json;
use crate::auth::jwt::validate_token;

pub struct AuthMiddleware;

// Transform implementation remains the same
impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService { service }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        if req.path() == "/auth" {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            });
        }

        let auth_header = req.headers().get(header::AUTHORIZATION);

        let auth_token = match auth_header {
            Some(header) => {
                match header.to_str() {
                    Ok(header_str) if header_str.starts_with("Bearer ") => {
                        header_str[7..].to_string()
                    },
                    _ => {
                        return Box::pin(async move {
                            let (request, _) = req.into_parts();
                            // Create the complete response first
                            let error_response = HttpResponse::Unauthorized()
                                .json(json!({ "error": "Invalid authorization header format" }));
                            // Then create the ServiceResponse and map it
                            Ok(ServiceResponse::new(
                                request,
                                error_response,
                            ).map_into_right_body())
                        });
                    }
                }
            }
            None => {
                return Box::pin(async move {
                    let (request, _) = req.into_parts();
                    let error_response = HttpResponse::Unauthorized()
                        .json(json!({ "error": "Missing authorization header" }));
                    Ok(ServiceResponse::new(
                        request,
                        error_response,
                    ).map_into_right_body())
                });
            }
        };

        match validate_token(auth_token) {
            Ok(claims) => {
                info!("Authenticated user: {}", claims.sub);
                let fut = self.service.call(req);
                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_left_body())
                })
            }
            Err(e) => {
                error!("Token validation failed: {:?}", e);
                Box::pin(async move {
                    let (request, _) = req.into_parts();
                    let error_response = HttpResponse::Unauthorized()
                        .json(json!({ "error": "Invalid token" }));
                    Ok(ServiceResponse::new(
                        request,
                        error_response,
                    ).map_into_right_body())
                })
            }
        }
    }
}