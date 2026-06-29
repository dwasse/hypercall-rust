//! Error types for the Hypercall client SDK.
//!
//! All fallible operations return [`Result<T>`](crate::error::Result) which uses
//! [`ClientError`] as the error type.
//!
//! # Error Variants
//!
//! - [`ClientError::Api`]: server returned an error (4xx/5xx with message)
//! - [`ClientError::Http`]: network/transport failure
//! - [`ClientError::Signing`]: EIP-712 signature creation failed
//! - [`ClientError::InvalidInput`]: bad input (unknown symbol, invalid key, etc)
//! - [`ClientError::WebSocket`]: WS connection or message failure

use std::fmt;

/// Client error type.
#[derive(Debug)]
pub enum ClientError {
    /// HTTP request failed
    Http(reqwest::Error),
    /// JSON serialization/deserialization failed
    Json(sonic_rs::Error),
    /// Signing failed
    Signing(String),
    /// API returned an error
    Api { status: u16, message: String },
    /// WebSocket error
    WebSocket(String),
    /// Invalid input
    InvalidInput(String),
    /// Other error
    Other(String),
}

impl ClientError {
    /// Returns true if this error indicates a connection-level failure
    /// (TCP connect error or request timeout), as opposed to an HTTP response error.
    pub fn is_connection_error(&self) -> bool {
        match self {
            Self::Http(e) => e.is_connect() || e.is_timeout(),
            _ => false,
        }
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(e) => write!(f, "HTTP error: {}", e),
            Self::Json(e) => write!(f, "JSON error: {}", e),
            Self::Signing(msg) => write!(f, "Signing error: {}", msg),
            Self::Api { status, message } => write!(f, "API error ({}): {}", status, message),
            Self::WebSocket(msg) => write!(f, "WebSocket error: {}", msg),
            Self::InvalidInput(msg) => write!(f, "invalid input: {}", msg),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(e) => Some(e),
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(e)
    }
}

impl From<sonic_rs::Error> for ClientError {
    fn from(e: sonic_rs::Error) -> Self {
        Self::Json(e)
    }
}

impl From<String> for ClientError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

impl From<&str> for ClientError {
    fn from(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}

/// Result type alias for client operations.
pub type Result<T> = std::result::Result<T, ClientError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_json() {
        let json_err: sonic_rs::Error = sonic_rs::from_str::<i32>("invalid").unwrap_err();
        let client_err = ClientError::Json(json_err);
        let display = format!("{}", client_err);
        assert!(display.starts_with("JSON error:"));
    }

    #[test]
    fn test_error_display_signing() {
        let err = ClientError::Signing("test signing error".to_string());
        assert_eq!(format!("{}", err), "Signing error: test signing error");
    }

    #[test]
    fn test_error_display_api() {
        let err = ClientError::Api {
            status: 404,
            message: "Not found".to_string(),
        };
        assert_eq!(format!("{}", err), "API error (404): Not found");
    }

    #[test]
    fn test_error_display_websocket() {
        let err = ClientError::WebSocket("connection closed".to_string());
        assert_eq!(format!("{}", err), "WebSocket error: connection closed");
    }

    #[test]
    fn test_error_display_other() {
        let err = ClientError::Other("some error".to_string());
        assert_eq!(format!("{}", err), "some error");
    }

    #[test]
    fn test_from_string() {
        let err: ClientError = "test error".to_string().into();
        match err {
            ClientError::Other(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Expected Other variant"),
        }
    }

    #[test]
    fn test_from_str() {
        let err: ClientError = "test error".into();
        match err {
            ClientError::Other(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Expected Other variant"),
        }
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err: sonic_rs::Error = sonic_rs::from_str::<i32>("not json").unwrap_err();
        let err: ClientError = json_err.into();
        assert!(matches!(err, ClientError::Json(_)));
    }

    #[test]
    fn test_error_source_json() {
        use std::error::Error;
        let json_err: sonic_rs::Error = sonic_rs::from_str::<i32>("invalid").unwrap_err();
        let client_err = ClientError::Json(json_err);
        assert!(client_err.source().is_some());
    }

    #[test]
    fn test_error_source_other_variants() {
        use std::error::Error;
        let signing_err = ClientError::Signing("test".to_string());
        assert!(signing_err.source().is_none());

        let api_err = ClientError::Api {
            status: 500,
            message: "error".to_string(),
        };
        assert!(api_err.source().is_none());

        let ws_err = ClientError::WebSocket("error".to_string());
        assert!(ws_err.source().is_none());

        let other_err = ClientError::Other("error".to_string());
        assert!(other_err.source().is_none());
    }

    #[test]
    fn test_is_connection_error_non_http_variants() {
        assert!(
            !ClientError::Json(sonic_rs::from_str::<i32>("x").unwrap_err()).is_connection_error()
        );
        assert!(!ClientError::Signing("s".into()).is_connection_error());
        assert!(!ClientError::Api {
            status: 500,
            message: "err".into()
        }
        .is_connection_error());
        assert!(!ClientError::WebSocket("ws".into()).is_connection_error());
        assert!(!ClientError::Other("other".into()).is_connection_error());
    }

    #[tokio::test]
    async fn test_is_connection_error_connect_failure() {
        // Attempt to connect to a port that's not listening, producing a connect error.
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_millis(50))
            .build()
            .unwrap();
        let err = client
            .get("http://127.0.0.1:1") // port 1 should be unreachable
            .send()
            .await
            .unwrap_err();
        let client_err = ClientError::Http(err);
        assert!(client_err.is_connection_error());
    }

    #[test]
    fn test_debug_impl() {
        let err = ClientError::Api {
            status: 400,
            message: "Bad request".to_string(),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("Api"));
        assert!(debug.contains("400"));
        assert!(debug.contains("Bad request"));
    }
}
