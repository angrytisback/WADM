use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;

#[allow(dead_code)]
#[derive(Debug)]
pub enum AppError {
    ExecutionFailed(String),
    Unsupported(String),
    NotFound(String),
    InvalidInput(String),
    Forbidden(String),
    Unauthorized(String),
    Io(std::io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            AppError::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Io(err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<String> for AppError {
    fn from(msg: String) -> Self {
        AppError::ExecutionFailed(msg)
    }
}

impl From<&str> for AppError {
    fn from(msg: &str) -> Self {
        AppError::ExecutionFailed(msg.to_string())
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl ResponseError for AppError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            AppError::NotFound(_) => actix_web::http::StatusCode::NOT_FOUND,
            AppError::InvalidInput(_) | AppError::Unsupported(_) => {
                actix_web::http::StatusCode::BAD_REQUEST
            }
            AppError::Forbidden(_) => actix_web::http::StatusCode::FORBIDDEN,
            AppError::Unauthorized(_) => actix_web::http::StatusCode::UNAUTHORIZED,
            AppError::ExecutionFailed(_) | AppError::Io(_) => {
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        let msg = match self {
            AppError::Forbidden(msg) => {
                if msg.is_empty() {
                    "Insufficient permissions".to_string()
                } else {
                    msg.clone()
                }
            }
            _ => self.to_string(),
        };
        HttpResponse::build(self.status_code()).json(ErrorBody { error: msg })
    }
}
