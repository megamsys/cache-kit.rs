//! Error handling for the cache-kit Actix example
//!
//! Inspired by Exonum API error patterns with structured error responses

use actix_web::{
    http::{header, StatusCode as HttpStatusCode},
    HttpResponse, ResponseError,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// API HTTP error struct
#[derive(Error, Debug)]
pub struct ApiError {
    /// HTTP status code
    pub http_code: HttpStatusCode,
    /// Error body
    pub body: ErrorBody,
}

/// Error body serialized in JSON responses
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ErrorBody {
    /// Short error title
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    /// Detailed error description
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// Error code for client handling
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<u16>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.body.detail.is_empty() {
            write!(f, "{}: {}", self.body.title, self.body.detail)
        } else {
            write!(f, "{}", self.body.title)
        }
    }
}

impl ApiError {
    /// Create new error with HTTP status code
    pub fn new(http_code: HttpStatusCode) -> Self {
        Self {
            http_code,
            body: ErrorBody::default(),
        }
    }

    /// Build Bad Request (400) error
    pub fn bad_request() -> Self {
        Self::new(HttpStatusCode::BAD_REQUEST).title("Bad Request")
    }

    /// Build Not Found (404) error
    pub fn not_found() -> Self {
        Self::new(HttpStatusCode::NOT_FOUND).title("Not Found")
    }

    /// Build Internal Server Error (500)
    pub fn internal(cause: impl fmt::Display) -> Self {
        Self::new(HttpStatusCode::INTERNAL_SERVER_ERROR)
            .title("Internal Server Error")
            .detail(cause.to_string())
    }

    /// Set error title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.body.title = title.into();
        self
    }

    /// Set error detail
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.body.detail = detail.into();
        self
    }

    /// Set error code
    pub fn error_code(mut self, code: u16) -> Self {
        self.body.error_code = Some(code);
        self
    }
}

/// Implement ResponseError for Actix integration
impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let body = serde_json::to_string(&self.body).unwrap_or_default();

        HttpResponse::build(self.http_code)
            .append_header((header::CONTENT_TYPE, "application/problem+json"))
            .body(body)
    }

    fn status_code(&self) -> HttpStatusCode {
        self.http_code
    }
}

/// Convert cache-kit errors to ApiError
impl From<cache_kit::error::Error> for ApiError {
    fn from(err: cache_kit::error::Error) -> Self {
        ApiError::internal(err)
    }
}

/// Convert String errors to ApiError
impl From<String> for ApiError {
    fn from(err: String) -> Self {
        ApiError::internal(err)
    }
}

/// Type alias for Results using ApiError
pub type Result<T> = std::result::Result<T, ApiError>;
