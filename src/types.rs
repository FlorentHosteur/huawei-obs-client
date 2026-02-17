//! Data types for OBS operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Information about a bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    /// Bucket name.
    pub name: String,
    /// When the bucket was created.
    pub creation_date: Option<SystemTime>,
    /// Region where the bucket resides.
    pub region: Option<String>,
}

/// Information about an object in a bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    /// Object key (path).
    pub key: String,
    /// Object size in bytes.
    pub size: u64,
    /// Last modification time.
    pub last_modified: Option<SystemTime>,
    /// ETag (content hash).
    pub etag: Option<String>,
    /// Storage class (e.g. STANDARD, WARM, COLD).
    pub storage_class: Option<String>,
}

/// Metadata for an object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    /// MIME content type.
    pub content_type: Option<String>,
    /// Size in bytes.
    pub content_length: u64,
    /// ETag (content hash).
    pub etag: Option<String>,
    /// Last modification time.
    pub last_modified: Option<SystemTime>,
    /// User-defined metadata key-value pairs.
    pub metadata: HashMap<String, String>,
}

/// Options for uploading an object.
#[derive(Debug, Clone, Default)]
pub struct UploadOptions {
    /// MIME content type (defaults to `application/octet-stream`).
    pub content_type: Option<String>,
    /// User-defined metadata.
    pub metadata: Option<HashMap<String, String>>,
    /// Storage class (e.g. "STANDARD", "WARM", "COLD").
    pub storage_class: Option<String>,
}

/// Options for listing objects.
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    /// Only return objects with this key prefix.
    pub prefix: Option<String>,
    /// Delimiter for grouping (typically `"/"`).
    pub delimiter: Option<String>,
    /// Maximum number of keys to return.
    pub max_keys: Option<i32>,
    /// Continuation token for pagination.
    pub continuation_token: Option<String>,
}

/// Result of listing objects in a bucket.
#[derive(Debug, Clone)]
pub struct ListResult {
    /// Objects matching the query.
    pub objects: Vec<ObjectInfo>,
    /// Common prefixes (virtual directories) when using a delimiter.
    pub common_prefixes: Vec<String>,
    /// Whether more results are available.
    pub is_truncated: bool,
    /// Token to fetch the next page of results.
    pub next_continuation_token: Option<String>,
}

/// Tag filter used in lifecycle rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleTag {
    /// Tag key.
    pub key: String,
    /// Tag value.
    pub value: String,
}

/// A lifecycle rule for automatic object management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleRule {
    /// Rule identifier.
    pub id: String,
    /// Key prefix filter.
    pub prefix: Option<String>,
    /// Whether the rule is active.
    pub enabled: bool,
    /// Days after creation to expire objects.
    pub expiration_days: Option<i32>,
    /// Days after becoming non-current to expire versions.
    pub noncurrent_version_expiration_days: Option<i32>,
    /// Days after creation to transition objects.
    pub transition_days: Option<i32>,
    /// Storage class to transition objects to.
    pub transition_storage_class: Option<String>,
    /// Tag filters (AND logic).
    pub tags: Option<Vec<LifecycleTag>>,
    /// Minimum object size filter in bytes.
    pub object_size_greater_than: Option<i64>,
    /// Maximum object size filter in bytes.
    pub object_size_less_than: Option<i64>,
}

/// A CORS rule for a bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsRule {
    /// Rule identifier.
    pub id: String,
    /// Allowed origins (e.g. `["*"]` or `["https://example.com"]`).
    pub allowed_origins: Vec<String>,
    /// Allowed HTTP methods (e.g. `["GET", "PUT"]`).
    pub allowed_methods: Vec<String>,
    /// Allowed request headers.
    pub allowed_headers: Option<Vec<String>>,
    /// Response headers exposed to the browser.
    pub expose_headers: Option<Vec<String>>,
    /// Max age for preflight cache in seconds.
    pub max_age_seconds: Option<i32>,
}

/// Bucket policy document (JSON).
#[derive(Debug, Clone)]
pub struct BucketPolicy {
    /// Raw JSON policy string.
    pub policy_json: String,
}

/// Public access configuration for a bucket.
#[derive(Debug, Clone)]
pub struct PublicAccessConfig {
    /// Whether public read is allowed.
    pub allow_public_read: bool,
    /// Whether public write is allowed.
    pub allow_public_write: bool,
}

/// WORM (Write Once Read Many) retention mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WormRetentionMode {
    /// Objects cannot be deleted or modified until retention expires.
    #[serde(rename = "COMPLIANCE")]
    Compliance,
}

/// WORM retention configuration for an individual object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WormRetention {
    /// Retention mode.
    pub mode: WormRetentionMode,
    /// Unix epoch timestamp (milliseconds) until which the object is protected.
    pub retain_until_date: i64,
}

/// Default WORM retention applied to new objects in a bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultWormRetention {
    /// Retention mode.
    pub mode: WormRetentionMode,
    /// Retention period in days (1–36500).
    pub days: Option<i32>,
    /// Retention period in years (1–100).
    pub years: Option<i32>,
}

/// Bucket-level Object Lock configuration.
#[derive(Debug, Clone)]
pub struct ObjectLockConfiguration {
    /// Whether Object Lock is enabled.
    pub enabled: bool,
    /// Default retention applied to new objects.
    pub default_retention: Option<DefaultWormRetention>,
}
