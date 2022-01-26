use std::collections::HashMap;
use std::task::{Context, Poll};

use actix_service::{Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpResponse};
use actix_web::http::header::AUTHORIZATION;
use actix_web::web::Query;
use futures::future::{Either, ok, Ready};

pub struct Auth {
    pub apikey: Option<String>
}

impl<S, B> Transform<S> for Auth
    where
        S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware { apikey: self.apikey.clone(), service })
    }
}

pub struct AuthMiddleware<S> {
    apikey: Option<String>,
    service: S,
}

impl<S, B> Service for AuthMiddleware<S>
    where
        S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Either<S::Future, Ready<Result<Self::Response, Self::Error>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        if self.apikey.is_some() {
            let mut is_authorized = false;
            let qs = Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
            if qs.contains_key("apikey") && qs.get("apikey").unwrap() == self.apikey.as_ref().unwrap() {
                is_authorized = true;
            }
            if req.headers().contains_key(AUTHORIZATION) &&
                req.headers().get(AUTHORIZATION).unwrap().to_str().unwrap() == self.apikey.as_ref().unwrap() {
                is_authorized = true;
            }
            if is_authorized {
                return Either::Left(self.service.call(req));
            }
            return Either::Right(
                ok(
                    req.into_response(HttpResponse::Unauthorized().finish().into_body())
                )
            );
        }
        return Either::Left(self.service.call(req));
    }
}