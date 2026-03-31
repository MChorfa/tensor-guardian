//! API error handling
//!
//! Maps domain errors to appropriate gRPC and HTTP status codes.

use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tonic::{Code, Status};

/// API result type
pub type Result<T> = std::result::Result<T, ApiError>;

/// API error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    NotFound,
    InvalidRequest,
    BackendUnavailable,
    CollectionFailed,
    InternalError,
}

impl ApiError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::NotFound,
            message: message.into(),
            details: None,
        }
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidRequest,
            message: message.into(),
            details: None,
        }
    }

    pub fn backend_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::BackendUnavailable,
            message: message.into(),
            details: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InternalError,
            message: message.into(),
            details: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code {
            ErrorCode::NotFound => axum::http::StatusCode::NOT_FOUND,
            ErrorCode::InvalidRequest => axum::http::StatusCode::BAD_REQUEST,
            ErrorCode::BackendUnavailable => axum::http::StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::CollectionFailed => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::InternalError => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}

impl From<ApiError> for Status {
    fn from(err: ApiError) -> Self {
        let code = match err.code {
            ErrorCode::NotFound => Code::NotFound,
            ErrorCode::InvalidRequest => Code::InvalidArgument,
            ErrorCode::BackendUnavailable => Code::Unavailable,
            ErrorCode::CollectionFailed => Code::Internal,
            ErrorCode::InternalError => Code::Internal,
        };

        Status::new(code, err.message)
    }
}

impl From<tensor_guardian_domain::DomainError> for ApiError {
    fn from(err: tensor_guardian_domain::DomainError) -> Self {
        match err {
            tensor_guardian_domain::DomainError::AcceleratorNotFound(id) => {
                ApiError::not_found(format!("Accelerator not found: {}", id))
            }
            tensor_guardian_domain::DomainError::SensorNotFound(id) => {
                ApiError::not_found(format!("Sensor not found: {}", id))
            }
            tensor_guardian_domain::DomainError::InvalidMetricValue(msg) => {
                ApiError::invalid_request(format!("Invalid metric value: {}", msg))
            }
            tensor_guardian_domain::DomainError::BackendError(msg) => {
                ApiError::backend_unavailable(format!("Backend error: {}", msg))
            }
            tensor_guardian_domain::DomainError::CollectionFailed(msg) => {
                ApiError::internal(format!("Collection failed: {}", msg))
            }
            tensor_guardian_domain::DomainError::ConfigurationError(msg) => {
                ApiError::invalid_request(format!("Configuration error: {}", msg))
            }
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::internal(err.to_string())
    }
}
