# huawei-obs-client

A Rust client library for **Huawei OBS** (Object Storage Service). Huawei OBS is S3-compatible, but has quirks (like requiring `Content-MD5` on CORS operations) that standard S3 libraries don't handle. This crate wraps those differences into a clean, async API.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
huawei-obs-client = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Quick Start

```rust
use huawei_obs_client::ObsClient;

#[tokio::main]
async fn main() -> huawei_obs_client::Result<()> {
    let client = ObsClient::builder()
        .access_key("your-access-key")
        .secret_key("your-secret-key")
        .endpoint("https://obs.ap-southeast-1.myhuaweicloud.com")
        .region("ap-southeast-1")
        .build()?;

    // List buckets
    let buckets = client.list_buckets().await?;
    for b in &buckets {
        println!("{}", b.name);
    }

    // Upload an object
    client
        .upload_object("my-bucket", "hello.txt", "Hello, world!".into(), None)
        .await?;

    // Download it back
    let data = client.download_object("my-bucket", "hello.txt").await?;
    println!("{}", String::from_utf8_lossy(&data));

    Ok(())
}
```

## API Overview

### Client Construction

```rust
// Builder pattern (recommended)
let client = ObsClient::builder()
    .access_key("AK")
    .secret_key("SK")
    .endpoint("https://obs.region.myhuaweicloud.com")
    .region("region-id")  // optional, defaults to "us-east-1"
    .build()?;

// Direct construction
let client = ObsClient::new(
    "access-key".into(),
    "secret-key".into(),
    "https://obs.region.myhuaweicloud.com".into(),
    "region-id".into(),
)?;
```

### Bucket Operations

```rust
// List all buckets
let buckets = client.list_buckets().await?;

// Create a bucket
client.create_bucket("new-bucket").await?;

// Check existence
let exists = client.bucket_exists("my-bucket").await?;

// Delete an empty bucket
client.delete_bucket("old-bucket").await?;
```

### Object Operations

```rust
use huawei_obs_client::{UploadOptions, ListOptions};

// Upload from bytes
client.upload_object("bucket", "key", data, None).await?;

// Upload with options
client.upload_object("bucket", "photo.jpg", data, Some(UploadOptions {
    content_type: Some("image/jpeg".into()),
    ..Default::default()
})).await?;

// Upload from a stream (e.g. a file)
let file = tokio::fs::File::open("large-file.bin").await?;
client.upload_object_stream("bucket", "key", Box::new(file), None).await?;

// Download
let data = client.download_object("bucket", "key").await?;

// Delete
client.delete_object("bucket", "key").await?;

// Batch delete
let deleted = client.delete_objects("bucket", vec!["a.txt".into(), "b.txt".into()]).await?;

// Check existence
let exists = client.object_exists("bucket", "key").await?;

// Get metadata
let meta = client.get_object_metadata("bucket", "key").await?;
println!("Size: {} bytes, Type: {:?}", meta.content_length, meta.content_type);

// List objects
let result = client.list_objects("bucket", ListOptions {
    prefix: Some("photos/".into()),
    delimiter: Some("/".into()),
    ..Default::default()
}).await?;

// Copy
client.copy_object("src-bucket", "src-key", "dst-bucket", "dst-key").await?;

// Presigned URLs
let get_url = client.presign_get("bucket", "key", 3600).await?;
let put_url = client.presign_put("bucket", "key", 3600).await?;
```

### Lifecycle Rules

```rust
use huawei_obs_client::LifecycleRule;

// Get current rules
let rules = client.get_lifecycle_rules("bucket").await?;

// Set rules
client.put_lifecycle_rules("bucket", vec![
    LifecycleRule {
        id: "expire-tmp".into(),
        prefix: Some("tmp/".into()),
        enabled: true,
        expiration_days: Some(7),
        ..Default::default()
    },
]).await?;

// Delete all rules
client.delete_lifecycle_rules("bucket").await?;
```

### CORS Configuration

```rust
use huawei_obs_client::CorsRule;

// Get
let rules = client.get_bucket_cors("bucket").await?;

// Set (Content-MD5 is computed automatically)
client.put_bucket_cors("bucket", vec![
    CorsRule {
        id: "allow-all".into(),
        allowed_origins: vec!["*".into()],
        allowed_methods: vec!["GET".into(), "PUT".into()],
        allowed_headers: Some(vec!["*".into()]),
        expose_headers: Some(vec!["ETag".into()]),
        max_age_seconds: Some(3000),
    },
]).await?;

// Delete
client.delete_bucket_cors("bucket").await?;
```

### Bucket Policy

```rust
use huawei_obs_client::BucketPolicy;

// Get
let policy = client.get_bucket_policy("bucket").await?;

// Set
client.put_bucket_policy("bucket", BucketPolicy {
    policy_json: serde_json::json!({
        "Version": "2012-10-17",
        "Statement": [{
            "Effect": "Allow",
            "Principal": "*",
            "Action": "s3:GetObject",
            "Resource": "arn:aws:s3:::my-bucket/*"
        }]
    }).to_string(),
}).await?;

// Delete
client.delete_bucket_policy("bucket").await?;
```

### Object Lock (WORM)

```rust
use huawei_obs_client::{ObjectLockConfiguration, DefaultWormRetention, WormRetentionMode};

// Get
let config = client.get_object_lock_configuration("bucket").await?;

// Enable with 30-day default retention
client.put_object_lock_configuration("bucket", ObjectLockConfiguration {
    enabled: true,
    default_retention: Some(DefaultWormRetention {
        mode: WormRetentionMode::Compliance,
        days: Some(30),
        years: None,
    }),
}).await?;
```

## Error Handling

All methods return `Result<T, ObsError>`. You can match on specific error variants:

```rust
use huawei_obs_client::ObsError;

match client.delete_bucket("test").await {
    Ok(_) => println!("Deleted!"),
    Err(ObsError::BucketNotEmpty(name)) => {
        eprintln!("Bucket '{}' is not empty — delete objects first", name);
    }
    Err(ObsError::BucketNotFound(name)) => {
        eprintln!("Bucket '{}' does not exist", name);
    }
    Err(ObsError::PermissionDenied(_)) => {
        eprintln!("Access denied — check your credentials");
    }
    Err(e) => {
        // user_message() returns a human-friendly string
        eprintln!("Error: {}", e.user_message());
    }
}
```

### Error Variants

| Variant | Meaning |
|---------|---------|
| `NotFound` | Object does not exist |
| `BucketNotFound` | Bucket does not exist |
| `BucketNotEmpty` | Cannot delete non-empty bucket |
| `BucketAlreadyExists` | Bucket name is taken |
| `PermissionDenied` | Access denied |
| `InvalidCredentials` | Bad AK/SK |
| `InvalidConfig` | Missing endpoint, etc. |
| `Network` | DNS, connection issues |
| `ConnectionFailed` | TCP connection failed |
| `Timeout` | Operation timed out |
| `S3Error` | S3 protocol error |
| `AwsSdk` | AWS SDK internal error |
| `Io` | Local filesystem error |
| `Other` | Unclassified error |

## Examples

Run the examples with your credentials:

```bash
export OBS_ACCESS_KEY=your-ak
export OBS_SECRET_KEY=your-sk
export OBS_ENDPOINT=https://obs.ap-southeast-1.myhuaweicloud.com

# Basic operations
cargo run --example basic

# Lifecycle rules
cargo run --example lifecycle -- my-bucket

# CORS management
cargo run --example cors -- my-bucket
```

## Architecture

The library uses two underlying S3 implementations:

- **[rust-s3](https://crates.io/crates/rust-s3)** — for basic object operations (upload, download, delete, list, presign). Handles Content-MD5 headers properly.
- **[aws-sdk-s3](https://crates.io/crates/aws-sdk-s3)** — for advanced operations (lifecycle, CORS, policies, bucket management) where its typed builders are more reliable.

Both are wrapped behind the unified `ObsClient` API, so you don't need to worry about which SDK handles what.

## License

MIT — see [LICENSE](LICENSE).
