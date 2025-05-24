use actix_web::http::StatusCode;
use actix_web::web::Json;
use actix_web::{HttpResponse, ResponseError};
use anyhow::{anyhow, Error};
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
}

impl From<&Error> for ErrorResponse {
    fn from(error: &Error) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
}

impl SuccessResponse {
    pub fn from(success: bool) -> Self {
        Self { success }
    }
}


pub type JsonResult<T> = Result<Json<T>, HttpError>;
pub type ResponseResult = Result<HttpResponse, HttpError>;

#[derive(Debug)]
pub struct HttpError {
    error: Error,
    status_code: StatusCode
}

impl HttpError {
    pub fn new(error: Error, status_code: StatusCode) -> Self {
        HttpError { error, status_code }
    }

    pub fn not_found() -> Self {
        Self::new(anyhow!("Not Found"), StatusCode::NOT_FOUND)
    }
}

impl Display for HttpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.error.to_string().as_str())
    }
}

impl ResponseError for HttpError {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code).json(ErrorResponse::from(&self.error))
    }
}

impl From<Error> for HttpError {
    fn from(error: Error) -> Self {
        Self { error, status_code: StatusCode::INTERNAL_SERVER_ERROR }
    }
}

impl From<reqwest::Error> for HttpError {
    fn from(error: reqwest::Error) -> Self {
        Self::from(<reqwest::Error as Into<Error>>::into(error))
    }
}