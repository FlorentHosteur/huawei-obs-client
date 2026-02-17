//! Error types for Huawei OBS operations.

use thiserror::Error;

/// Errors that can occur during OBS operations.
#[derive(Debug, Error)]
pub enum ObsError {
    /// Object not found in the bucket.
    #[error("Object not found: {0}")]
    NotFound(String),

    /// Bucket does not exist or is inaccessible.
    #[error("Bucket not found: {0}")]
    BucketNotFound(String),

    /// Access denied — check credentials and permissions.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid configuration (e.g. missing endpoint).
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Network-level error (DNS, connection refused, etc.).
    #[error("Network error: {0}")]
    Network(String),

    /// Error from the AWS SDK layer.
    #[error("AWS SDK error: {0}")]
    AwsSdk(String),

    /// Error from the rust-s3 layer.
    #[error("S3 error: {0}")]
    S3Error(String),

    /// Local I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Bucket is not empty (cannot delete).
    #[error("Bucket not empty: {0}")]
    BucketNotEmpty(String),

    /// Bucket already exists.
    #[error("Bucket already exists: {0}")]
    BucketAlreadyExists(String),

    /// Invalid access key or secret key.
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// TCP connection failed.
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Operation timed out.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Catch-all for unclassified errors.
    #[error("Error: {0}")]
    Other(String),
}

impl ObsError {
    /// Returns a human-friendly error message suitable for end users.
    ///
    /// # Example
    ///
    /// ```
    /// use huawei_obs_client::ObsError;
    ///
    /// let err = ObsError::BucketNotFound("my-bucket".into());
    /// assert!(err.user_message().contains("my-bucket"));
    /// ```
    pub fn user_message(&self) -> String {
        match self {
            ObsError::NotFound(resource) => {
                if resource.is_empty() {
                    "The requested file or folder does not exist.".to_string()
                } else {
                    format!("'{}' does not exist.", resource)
                }
            }
            ObsError::BucketNotFound(bucket) => {
                format!(
                    "Bucket '{}' does not exist or you don't have access to it.",
                    bucket
                )
            }
            ObsError::PermissionDenied(_) => {
                "Access denied. Check your credentials and permissions.".to_string()
            }
            ObsError::InvalidConfig(msg) => {
                format!("Configuration error: {}", msg)
            }
            ObsError::Network(msg) => {
                let lower = msg.to_lowercase();
                if lower.contains("timeout") || lower.contains("timed out") {
                    "Connection timed out. Check your network connection.".to_string()
                } else if lower.contains("refused") {
                    "Connection refused. The server may be unavailable.".to_string()
                } else if lower.contains("resolve") || lower.contains("dns") {
                    "Could not reach the server. Check the endpoint URL.".to_string()
                } else {
                    "Network error. Check your internet connection.".to_string()
                }
            }
            ObsError::AwsSdk(msg) | ObsError::S3Error(msg) => Self::parse_s3_error(msg),
            ObsError::Io(err) => match err.kind() {
                std::io::ErrorKind::NotFound => "File not found on local disk.".to_string(),
                std::io::ErrorKind::PermissionDenied => {
                    "Permission denied for local file.".to_string()
                }
                _ => format!("File error: {}", err),
            },
            ObsError::BucketNotEmpty(bucket) => {
                format!(
                    "Bucket '{}' is not empty. Delete all objects first.",
                    bucket
                )
            }
            ObsError::BucketAlreadyExists(bucket) => {
                format!("Bucket '{}' already exists.", bucket)
            }
            ObsError::InvalidCredentials => {
                "Invalid credentials. Check your access key and secret key.".to_string()
            }
            ObsError::ConnectionFailed(msg) => {
                if msg.is_empty() {
                    "Failed to connect to the server.".to_string()
                } else {
                    format!("Connection failed: {}", msg)
                }
            }
            ObsError::Timeout(_) => {
                "Operation timed out. Try again or check your connection.".to_string()
            }
            ObsError::Other(msg) => {
                if msg.is_empty() {
                    "Unknown error occurred.".to_string()
                } else {
                    Self::parse_s3_error(msg)
                }
            }
        }
    }

    /// Attempt to extract a useful message from an S3/AWS error string.
    fn parse_s3_error(msg: &str) -> String {
        let lower = msg.to_lowercase();

        if lower.contains("accessdenied") || lower.contains("access denied") {
            return "Access denied. Check your credentials and permissions.".to_string();
        }
        if lower.contains("invalidaccesskeyid") {
            return "Invalid access key. Check your credentials.".to_string();
        }
        if lower.contains("signaturemismatch") {
            return "Authentication failed. Check your secret key.".to_string();
        }
        if lower.contains("nosuchbucket") {
            return "Bucket does not exist.".to_string();
        }
        if lower.contains("bucketalreadyexists") {
            return "Bucket already exists.".to_string();
        }
        if lower.contains("bucketnotempty") {
            return "Bucket is not empty. Delete all objects first.".to_string();
        }
        if lower.contains("nosuchkey") {
            return "Object does not exist.".to_string();
        }
        if lower.contains("entitytoolarge") {
            return "File is too large for upload.".to_string();
        }
        if lower.contains("timeout") || lower.contains("timed out") {
            return "Operation timed out. Check your connection.".to_string();
        }
        if lower.contains("slowdown") || lower.contains("throttl") {
            return "Too many requests. Please wait and try again.".to_string();
        }
        if lower.contains("malformedpolicy") {
            return "Invalid bucket policy format.".to_string();
        }
        if lower.contains("nosuchcorsconfiguration") {
            return "No CORS configuration found for this bucket.".to_string();
        }

        msg.to_string()
    }
}

/// Result type alias for OBS operations.
pub type Result<T> = std::result::Result<T, ObsError>;
