use std::fmt;

use reqwest::header::HeaderMap;
use serde::Deserialize;
use serde_json::Value;

use crate::generated::client::{ApiOpError, HttpError};

/// A provider error body: `{"code": 404, "message": "...", "status": "error"}`.
///
/// Twelve Data sends this shape with the matching HTTP status, inside an HTTP 200
/// response, and per symbol inside batch quote responses.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ApiMessage {
    pub code: i64,
    pub message: String,
}

impl ApiMessage {
    pub(crate) fn from_value(value: &Value) -> Option<Self> {
        if value.get("status")?.as_str()? != "error" {
            return None;
        }
        serde_json::from_value(value.clone()).ok()
    }

    pub(crate) fn from_body(body: &str) -> Option<Self> {
        Self::from_value(&serde_json::from_str(body).ok()?)
    }
}

impl fmt::Display for ApiMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiMessage {}

/// Errors returned by the convenience client.
#[derive(Debug)]
pub enum Error {
    /// Transport, request construction or response-body limit failure.
    Transport(HttpError),
    /// A response the provider reported as an error, by HTTP status or in the body.
    Api {
        /// HTTP status; 200 when the error was only reported in the body.
        status: u16,
        /// Provider error code from the body, usually the intended HTTP status.
        code: Option<i64>,
        /// Provider error message from the body.
        message: Option<String>,
        /// Complete response body.
        body: String,
        /// Raw Retry-After header; may be seconds or an HTTP date.
        retry_after: Option<String>,
    },
    /// A success response that could not be decoded according to the schema.
    Decode(String),
    /// Invalid arguments rejected before making a request.
    InvalidRequest(&'static str),
}

impl Error {
    pub(crate) fn from_response(
        status: u16,
        body: String,
        headers: &HeaderMap,
        parse_error: Option<String>,
    ) -> Self {
        let message = ApiMessage::from_body(&body);
        if (200..300).contains(&status) && message.is_none() {
            return Self::Decode(
                parse_error.unwrap_or_else(|| "response body was not valid JSON".to_owned()),
            );
        }
        Self::Api {
            status,
            code: message.as_ref().map(|m| m.code),
            message: message.map(|m| m.message),
            body,
            retry_after: headers
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned),
        }
    }

    /// The provider code when present, otherwise the HTTP status.
    fn effective_status(&self) -> Option<i64> {
        match self {
            Self::Api { status, code, .. } => Some(code.unwrap_or(i64::from(*status))),
            _ => None,
        }
    }

    pub fn is_unauthorized(&self) -> bool {
        matches!(self.effective_status(), Some(401 | 403))
    }

    pub fn is_rate_limited(&self) -> bool {
        self.effective_status() == Some(429)
    }

    pub fn is_not_found(&self) -> bool {
        self.effective_status() == Some(404)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => write!(f, "transport error: {error}"),
            Self::Api {
                status,
                code,
                message,
                body,
                ..
            } => {
                write!(f, "Twelve Data responded with HTTP {status}")?;
                if let Some(code) = code {
                    write!(f, " (code {code})")?;
                }
                write!(f, ": {}", message.as_deref().unwrap_or(body))
            }
            Self::Decode(reason) => {
                write!(f, "Twelve Data response did not match the schema: {reason}")
            }
            Self::InvalidRequest(reason) => write!(f, "invalid request: {reason}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            _ => None,
        }
    }
}

impl From<HttpError> for Error {
    fn from(error: HttpError) -> Self {
        Self::Transport(error)
    }
}

impl<E: fmt::Debug> From<ApiOpError<E>> for Error {
    fn from(error: ApiOpError<E>) -> Self {
        match error {
            ApiOpError::Transport(error) => Self::Transport(error),
            ApiOpError::Api(error) => {
                Self::from_response(error.status, error.body, &error.headers, error.parse_error)
            }
        }
    }
}
