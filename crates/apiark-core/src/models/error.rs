use serde::{Deserialize, Serialize};

/// Internal typed errors for the HTTP engine.
#[derive(Debug, thiserror::Error)]
pub enum HttpEngineError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("DNS resolution failed: {0}")]
    DnsFailure(String),

    #[error("Connection refused: {0}")]
    ConnectionRefused(String),

    #[error("Connection timeout after {0}ms")]
    Timeout(u64),

    #[error("TLS/SSL error: {0}")]
    TlsError(String),

    #[error("Request error: {0}")]
    RequestError(String),

    #[error("Response too large (>{0} bytes)")]
    ResponseTooLarge(u64),

    #[error("Response body decode error: {0}")]
    BodyDecodeError(String),
}

/// Serializable error for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpError {
    pub error_type: String,
    pub message: String,
    pub suggestion: Option<String>,
}

impl From<HttpEngineError> for HttpError {
    fn from(err: HttpEngineError) -> Self {
        let (error_type, suggestion) = match &err {
            HttpEngineError::InvalidUrl(_) => (
                "invalid_url",
                Some("Check the URL format. It should start with http:// or https://".into()),
            ),
            HttpEngineError::DnsFailure(_) => (
                "dns_failure",
                Some("Could not resolve the hostname. Check the URL or your network connection.".into()),
            ),
            HttpEngineError::ConnectionRefused(_) => (
                "connection_refused",
                Some("Is the server running? Check the host and port.".into()),
            ),
            HttpEngineError::Timeout(ms) => (
                "timeout",
                Some(format!("Request timed out after {}ms. The server may be slow or unreachable.", ms)),
            ),
            HttpEngineError::TlsError(_) => (
                "tls_error",
                Some("SSL/TLS handshake failed. Try disabling SSL verification for development servers.".into()),
            ),
            HttpEngineError::RequestError(_) => ("request_error", None),
            HttpEngineError::ResponseTooLarge(_) => (
                "response_too_large",
                Some("The response exceeds the 10MB display limit.".into()),
            ),
            HttpEngineError::BodyDecodeError(_) => ("body_decode_error", None),
        };

        HttpError {
            error_type: error_type.to_string(),
            message: err.to_string(),
            suggestion,
        }
    }
}
