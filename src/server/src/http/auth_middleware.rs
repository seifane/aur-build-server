use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::error::ErrorUnauthorized;
use actix_web::http::header;
use actix_web::Error;
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

pub struct Auth {
    api_key: String,
}

impl Auth {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware { service, api_key: self.api_key.clone() }))
    }
}

pub struct AuthMiddleware<S> {
    service: S,
    api_key: String,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_header = req.headers().get(header::AUTHORIZATION);
        if let Some(auth_header) = auth_header {
            if auth_header == &self.api_key {
                return Box::pin(self.service.call(req));
            }
        }

        Box::pin(ready(Err(ErrorUnauthorized("Unauthorized"))))
    }
}