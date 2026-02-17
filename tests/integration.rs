//! Integration tests for huawei-obs-client against a live OBS endpoint.
//!
//! Run with:
//! ```bash
//! OBS_ACCESS_KEY=... OBS_SECRET_KEY=... OBS_ENDPOINT=https://obj.hosteur.io \
//!   OBS_BUCKET=hst-test-api-bk01 cargo test --test integration -- --ignored --test-threads=1
//! ```
//!
//! Tests run sequentially (--test-threads=1) to avoid conflicts on shared bucket.
//! Live tests are marked `#[ignore]` so `cargo test` passes without credentials.

use huawei_obs_client::*;
use std::time::Duration;

fn get_client() -> ObsClient {
    let ak = std::env::var("OBS_ACCESS_KEY").expect("OBS_ACCESS_KEY required");
    let sk = std::env::var("OBS_SECRET_KEY").expect("OBS_SECRET_KEY required");
    let endpoint = std::env::var("OBS_ENDPOINT").expect("OBS_ENDPOINT required");
    let region = std::env::var("OBS_REGION").unwrap_or_else(|_| "us-east-1".to_string());

    ObsClient::builder()
        .access_key(ak)
        .secret_key(sk)
        .endpoint(endpoint)
        .region(region)
        .build()
        .expect("Failed to build ObsClient")
}

fn test_bucket() -> String {
    std::env::var("OBS_BUCKET").expect("OBS_BUCKET required")
}

/// Unique prefix for this test run to avoid collisions.
fn test_prefix() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("__test_{}__/", ts)
}

// ═══════════════════════════════════════════════════════════════════════════
// Builder & Config Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_builder_missing_access_key() {
    let result = ObsClient::builder()
        .secret_key("sk")
        .endpoint("https://example.com")
        .build();
    assert!(result.is_err());
    match result {
        Err(ObsError::InvalidConfig(msg)) => assert!(msg.contains("access_key")),
        other => panic!("Expected InvalidConfig, got {:?}", other.err()),
    }
}

#[test]
fn test_builder_missing_secret_key() {
    let result = ObsClient::builder()
        .access_key("ak")
        .endpoint("https://example.com")
        .build();
    match result {
        Err(ObsError::InvalidConfig(msg)) => assert!(msg.contains("secret_key")),
        other => panic!("Expected InvalidConfig, got {:?}", other.err()),
    }
}

#[test]
fn test_builder_missing_endpoint() {
    let result = ObsClient::builder()
        .access_key("ak")
        .secret_key("sk")
        .build();
    match result {
        Err(ObsError::InvalidConfig(msg)) => assert!(msg.contains("endpoint")),
        other => panic!("Expected InvalidConfig, got {:?}", other.err()),
    }
}

#[test]
fn test_builder_valid() {
    let result = ObsClient::builder()
        .access_key("ak")
        .secret_key("sk")
        .endpoint("https://example.com")
        .region("us-east-1")
        .build();
    assert!(result.is_ok());
}

#[test]
fn test_new_direct() {
    let result = ObsClient::new(
        "ak".into(),
        "sk".into(),
        "https://example.com".into(),
        "us-east-1".into(),
    );
    assert!(result.is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════
// Error Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_error_user_message_not_found() {
    let err = ObsError::NotFound("file.txt".into());
    assert!(err.user_message().contains("file.txt"));
}

#[test]
fn test_error_user_message_empty_not_found() {
    let err = ObsError::NotFound(String::new());
    assert!(err.user_message().contains("does not exist"));
}

#[test]
fn test_error_user_message_bucket_not_found() {
    let err = ObsError::BucketNotFound("my-bucket".into());
    assert!(err.user_message().contains("my-bucket"));
}

#[test]
fn test_error_user_message_bucket_not_empty() {
    let err = ObsError::BucketNotEmpty("my-bucket".into());
    assert!(err.user_message().contains("not empty"));
}

#[test]
fn test_error_user_message_permission_denied() {
    let err = ObsError::PermissionDenied(String::new());
    assert!(err.user_message().contains("Access denied"));
}

#[test]
fn test_error_user_message_invalid_credentials() {
    let err = ObsError::InvalidCredentials;
    assert!(err.user_message().contains("credentials"));
}

#[test]
fn test_error_user_message_timeout() {
    let err = ObsError::Timeout(String::new());
    assert!(err.user_message().contains("timed out"));
}

#[test]
fn test_error_user_message_connection_failed() {
    let err = ObsError::ConnectionFailed(String::new());
    assert!(err.user_message().contains("connect"));
}

#[test]
fn test_error_user_message_network_timeout() {
    let err = ObsError::Network("connection timed out".into());
    assert!(err.user_message().contains("timed out"));
}

#[test]
fn test_error_user_message_network_refused() {
    let err = ObsError::Network("connection refused".into());
    assert!(err.user_message().contains("refused"));
}

#[test]
fn test_error_user_message_network_dns() {
    let err = ObsError::Network("DNS resolution failed".into());
    assert!(err.user_message().contains("endpoint"));
}

#[test]
fn test_error_display() {
    let err = ObsError::NotFound("key.txt".into());
    let display = format!("{}", err);
    assert!(display.contains("key.txt"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Bucket Operations (live)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn test_list_buckets() {
    let client = get_client();
    let buckets = client.list_buckets().await.expect("list_buckets failed");
    assert!(!buckets.is_empty(), "Should have at least one bucket");

    let bucket = test_bucket();
    let found = buckets.iter().any(|b| b.name == bucket);
    assert!(found, "Test bucket '{}' not found in bucket list", bucket);

    // Verify BucketInfo fields
    let test_b = buckets.iter().find(|b| b.name == bucket).unwrap();
    assert_eq!(test_b.name, bucket);
    assert!(test_b.creation_date.is_some(), "creation_date should be set");
}

#[tokio::test]
#[ignore]
async fn test_bucket_exists_true() {
    let client = get_client();
    let exists = client
        .bucket_exists(&test_bucket())
        .await
        .expect("bucket_exists failed");
    assert!(exists, "Test bucket should exist");
}

#[tokio::test]
#[ignore]
async fn test_bucket_exists_false() {
    let client = get_client();
    let exists = client
        .bucket_exists("nonexistent-bucket-xyzzy-12345")
        .await
        .expect("bucket_exists failed");
    assert!(!exists, "Random bucket should not exist");
}

#[tokio::test]
#[ignore]
async fn test_create_and_delete_bucket() {
    let client = get_client();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let name = format!("obs-test-tmp-{}", ts);

    // Create
    client
        .create_bucket(&name)
        .await
        .expect("create_bucket failed");

    // Verify exists
    let exists = client.bucket_exists(&name).await.expect("bucket_exists");
    assert!(exists, "Newly created bucket should exist");

    // Delete
    client
        .delete_bucket(&name)
        .await
        .expect("delete_bucket failed");

    // Small delay for consistency
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify gone
    let exists = client.bucket_exists(&name).await.expect("bucket_exists");
    assert!(!exists, "Deleted bucket should not exist");
}

#[tokio::test]
#[ignore]
async fn test_delete_nonexistent_bucket() {
    let client = get_client();
    let result = client.delete_bucket("nonexistent-bucket-xyzzy-99999").await;
    assert!(result.is_err(), "Deleting nonexistent bucket should fail");
}

// ═══════════════════════════════════════════════════════════════════════════
// Object Operations (live)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn test_upload_and_download_object() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();
    let key = format!("{}hello.txt", prefix);
    let content = b"Hello, OBS integration test!";

    // Upload
    client
        .upload_object(&bucket, &key, content.to_vec().into(), None)
        .await
        .expect("upload_object failed");

    // Download
    let data = client
        .download_object(&bucket, &key)
        .await
        .expect("download_object failed");
    assert_eq!(data.as_ref(), content, "Downloaded content should match");

    // Cleanup
    client.delete_object(&bucket, &key).await.expect("cleanup");
}

#[tokio::test]
#[ignore]
async fn test_upload_with_content_type() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();
    let key = format!("{}typed.json", prefix);

    client
        .upload_object(
            &bucket,
            &key,
            br#"{"test": true}"#.to_vec().into(),
            Some(UploadOptions {
                content_type: Some("application/json".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("upload with content_type failed");

    // Verify metadata
    let meta = client
        .get_object_metadata(&bucket, &key)
        .await
        .expect("get_object_metadata");
    assert_eq!(
        meta.content_type.as_deref(),
        Some("application/json"),
        "Content-Type should be application/json"
    );

    client.delete_object(&bucket, &key).await.expect("cleanup");
}

#[tokio::test]
#[ignore]
async fn test_upload_stream() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();
    let key = format!("{}streamed.bin", prefix);
    let content = vec![0x42u8; 5000];

    let cursor = std::io::Cursor::new(content.clone());
    client
        .upload_object_stream(&bucket, &key, Box::new(cursor), Some("application/octet-stream"))
        .await
        .expect("upload_object_stream failed");

    let data = client
        .download_object(&bucket, &key)
        .await
        .expect("download");
    assert_eq!(data.len(), content.len(), "Streamed upload size should match");
    assert_eq!(data.as_ref(), content.as_slice());

    client.delete_object(&bucket, &key).await.expect("cleanup");
}

#[tokio::test]
#[ignore]
async fn test_download_nonexistent_object() {
    let client = get_client();
    let result = client
        .download_object(&test_bucket(), "__nonexistent_key_xyz__")
        .await;
    assert!(result.is_err(), "Downloading nonexistent object should fail");
}

#[tokio::test]
#[ignore]
async fn test_object_exists() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();
    let key = format!("{}exists_check.txt", prefix);

    // Should not exist yet
    let exists = client
        .object_exists(&bucket, &key)
        .await
        .expect("object_exists");
    assert!(!exists, "Object should not exist before upload");

    // Upload
    client
        .upload_object(&bucket, &key, b"data".to_vec().into(), None)
        .await
        .expect("upload");

    // Should exist now
    let exists = client
        .object_exists(&bucket, &key)
        .await
        .expect("object_exists");
    assert!(exists, "Object should exist after upload");

    // Cleanup
    client.delete_object(&bucket, &key).await.expect("cleanup");
}

#[tokio::test]
#[ignore]
async fn test_get_object_metadata() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();
    let key = format!("{}metadata_test.txt", prefix);
    let content = b"metadata test content 12345";

    client
        .upload_object(
            &bucket,
            &key,
            content.to_vec().into(),
            Some(UploadOptions {
                content_type: Some("text/plain".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("upload");

    let meta = client
        .get_object_metadata(&bucket, &key)
        .await
        .expect("get_object_metadata");

    assert_eq!(meta.content_length, content.len() as u64);
    assert_eq!(meta.content_type.as_deref(), Some("text/plain"));
    assert!(meta.etag.is_some(), "ETag should be present");

    client.delete_object(&bucket, &key).await.expect("cleanup");
}

#[tokio::test]
#[ignore]
async fn test_delete_object() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();
    let key = format!("{}to_delete.txt", prefix);

    client
        .upload_object(&bucket, &key, b"delete me".to_vec().into(), None)
        .await
        .expect("upload");

    client
        .delete_object(&bucket, &key)
        .await
        .expect("delete_object failed");

    // Allow eventual consistency
    tokio::time::sleep(Duration::from_secs(2)).await;

    let exists = client
        .object_exists(&bucket, &key)
        .await
        .expect("object_exists");
    assert!(!exists, "Object should not exist after deletion");
}

#[tokio::test]
#[ignore]
async fn test_delete_objects_batch() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();

    let keys: Vec<String> = (0..5)
        .map(|i| format!("{}batch_{}.txt", prefix, i))
        .collect();

    // Upload all
    for key in &keys {
        client
            .upload_object(&bucket, key, b"batch".to_vec().into(), None)
            .await
            .expect("upload");
    }

    // Batch delete
    let deleted = client
        .delete_objects(&bucket, keys.clone())
        .await
        .expect("delete_objects failed");
    assert_eq!(deleted.len(), 5, "All 5 objects should be deleted");

    // Allow eventual consistency
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify all gone
    for key in &keys {
        let exists = client.object_exists(&bucket, key).await.expect("exists");
        assert!(!exists, "Object {} should be deleted", key);
    }
}

#[tokio::test]
#[ignore]
async fn test_list_objects() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();

    // Upload 3 objects
    for i in 0..3 {
        let key = format!("{}list_{}.txt", prefix, i);
        client
            .upload_object(&bucket, &key, format!("item {}", i).into(), None)
            .await
            .expect("upload");
    }

    // List with prefix
    let result = client
        .list_objects(
            &bucket,
            ListOptions {
                prefix: Some(prefix.clone()),
                ..Default::default()
            },
        )
        .await
        .expect("list_objects failed");

    assert_eq!(result.objects.len(), 3, "Should list exactly 3 objects");

    // Verify ObjectInfo fields
    for obj in &result.objects {
        assert!(obj.key.starts_with(&prefix));
        assert!(obj.size > 0);
    }

    // Cleanup
    let keys: Vec<String> = result.objects.iter().map(|o| o.key.clone()).collect();
    client.delete_objects(&bucket, keys).await.expect("cleanup");
}

#[tokio::test]
#[ignore]
async fn test_list_objects_with_delimiter() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();

    // Create "directories"
    let keys = vec![
        format!("{}dir_a/file1.txt", prefix),
        format!("{}dir_a/file2.txt", prefix),
        format!("{}dir_b/file3.txt", prefix),
        format!("{}top_level.txt", prefix),
    ];

    for key in &keys {
        client
            .upload_object(&bucket, key, b"x".to_vec().into(), None)
            .await
            .expect("upload");
    }

    // List with delimiter — should get common prefixes
    let result = client
        .list_objects(
            &bucket,
            ListOptions {
                prefix: Some(prefix.clone()),
                delimiter: Some("/".into()),
                ..Default::default()
            },
        )
        .await
        .expect("list_objects with delimiter");

    assert_eq!(
        result.objects.len(),
        1,
        "Should have 1 top-level object"
    );
    assert_eq!(
        result.common_prefixes.len(),
        2,
        "Should have 2 common prefixes (dir_a/, dir_b/)"
    );

    // Cleanup
    client.delete_objects(&bucket, keys).await.expect("cleanup");
}

#[tokio::test]
#[ignore]
async fn test_copy_object_same_bucket() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();
    let src_key = format!("{}copy_src.txt", prefix);
    let dst_key = format!("{}copy_dst.txt", prefix);
    let content = b"copy me!";

    client
        .upload_object(&bucket, &src_key, content.to_vec().into(), None)
        .await
        .expect("upload src");

    client
        .copy_object(&bucket, &src_key, &bucket, &dst_key)
        .await
        .expect("copy_object failed");

    // Verify copy
    let data = client
        .download_object(&bucket, &dst_key)
        .await
        .expect("download dst");
    assert_eq!(data.as_ref(), content);

    // Cleanup
    client
        .delete_objects(&bucket, vec![src_key, dst_key])
        .await
        .expect("cleanup");
}

#[tokio::test]
#[ignore]
async fn test_upload_empty_object() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();
    let key = format!("{}empty.bin", prefix);

    client
        .upload_object(&bucket, &key, bytes::Bytes::new(), None)
        .await
        .expect("upload empty object");

    let data = client
        .download_object(&bucket, &key)
        .await
        .expect("download");
    assert_eq!(data.len(), 0, "Empty object should have 0 bytes");

    let meta = client
        .get_object_metadata(&bucket, &key)
        .await
        .expect("metadata");
    assert_eq!(meta.content_length, 0);

    client.delete_object(&bucket, &key).await.expect("cleanup");
}

#[tokio::test]
#[ignore]
async fn test_upload_large_object() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();
    let key = format!("{}large.bin", prefix);

    // 1 MB
    let content = vec![0xABu8; 1_000_000];
    client
        .upload_object(&bucket, &key, content.clone().into(), None)
        .await
        .expect("upload large object");

    let data = client
        .download_object(&bucket, &key)
        .await
        .expect("download large");
    assert_eq!(data.len(), 1_000_000);
    assert_eq!(data[0], 0xAB);
    assert_eq!(data[999_999], 0xAB);

    client.delete_object(&bucket, &key).await.expect("cleanup");
}

// ═══════════════════════════════════════════════════════════════════════════
// Presigned URLs
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn test_presign_get() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();
    let key = format!("{}presign_get.txt", prefix);
    let content = b"presigned content";

    client
        .upload_object(&bucket, &key, content.to_vec().into(), None)
        .await
        .expect("upload");

    let url = client
        .presign_get(&bucket, &key, 3600)
        .await
        .expect("presign_get failed");

    assert!(url.starts_with("https://") || url.starts_with("http://"));
    assert!(url.contains(&key), "URL should contain the object key");
    assert!(
        url.contains("Signature") || url.contains("X-Amz-Signature"),
        "URL should contain signature"
    );

    // Actually fetch via the presigned URL
    let http = reqwest::Client::new();
    let resp = http.get(&url).send().await.expect("HTTP GET presigned");
    assert!(resp.status().is_success(), "Presigned GET should succeed");
    let body = resp.bytes().await.expect("read body");
    assert_eq!(body.as_ref(), content);

    client.delete_object(&bucket, &key).await.expect("cleanup");
}

#[tokio::test]
#[ignore]
async fn test_presign_put() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();
    let key = format!("{}presign_put.txt", prefix);

    let url = client
        .presign_put(&bucket, &key, 3600)
        .await
        .expect("presign_put failed");

    assert!(url.starts_with("https://") || url.starts_with("http://"));
    assert!(
        url.contains("Signature") || url.contains("X-Amz-Signature"),
        "URL should contain signature"
    );

    // Upload via presigned URL
    let http = reqwest::Client::new();
    let resp = http
        .put(&url)
        .body("presigned upload content")
        .send()
        .await
        .expect("HTTP PUT presigned");
    assert!(resp.status().is_success(), "Presigned PUT should succeed");

    // Verify the object was created
    let data = client
        .download_object(&bucket, &key)
        .await
        .expect("download");
    assert_eq!(data.as_ref(), b"presigned upload content");

    client.delete_object(&bucket, &key).await.expect("cleanup");
}

// ═══════════════════════════════════════════════════════════════════════════
// CORS Management
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn test_cors_crud() {
    let client = get_client();
    let bucket = test_bucket();

    // Delete existing CORS (clean state)
    let _ = client.delete_bucket_cors(&bucket).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // GET — should be empty
    let rules = client
        .get_bucket_cors(&bucket)
        .await
        .expect("get_bucket_cors");
    assert!(rules.is_empty(), "CORS should be empty initially");

    // PUT
    let cors = CorsRule {
        id: "test-rule".into(),
        allowed_origins: vec!["https://example.com".into()],
        allowed_methods: vec!["GET".into(), "PUT".into()],
        allowed_headers: Some(vec!["*".into()]),
        expose_headers: Some(vec!["ETag".into()]),
        max_age_seconds: Some(3000),
    };

    client
        .put_bucket_cors(&bucket, vec![cors])
        .await
        .expect("put_bucket_cors failed");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // GET — verify
    let rules = client
        .get_bucket_cors(&bucket)
        .await
        .expect("get_bucket_cors after put");
    assert_eq!(rules.len(), 1, "Should have 1 CORS rule");

    let rule = &rules[0];
    assert_eq!(rule.allowed_origins, vec!["https://example.com"]);
    assert_eq!(rule.allowed_methods, vec!["GET", "PUT"]);
    assert_eq!(rule.allowed_headers, Some(vec!["*".into()]));
    assert_eq!(rule.expose_headers, Some(vec!["ETag".into()]));
    assert_eq!(rule.max_age_seconds, Some(3000));

    // DELETE
    client
        .delete_bucket_cors(&bucket)
        .await
        .expect("delete_bucket_cors");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify deleted
    let rules = client
        .get_bucket_cors(&bucket)
        .await
        .expect("get_bucket_cors after delete");
    assert!(rules.is_empty(), "CORS should be empty after delete");
}

#[tokio::test]
#[ignore]
async fn test_cors_multiple_rules() {
    let client = get_client();
    let bucket = test_bucket();

    let _ = client.delete_bucket_cors(&bucket).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let rules = vec![
        CorsRule {
            id: "rule-1".into(),
            allowed_origins: vec!["https://app1.example.com".into()],
            allowed_methods: vec!["GET".into()],
            allowed_headers: None,
            expose_headers: None,
            max_age_seconds: Some(1800),
        },
        CorsRule {
            id: "rule-2".into(),
            allowed_origins: vec!["*".into()],
            allowed_methods: vec!["GET".into(), "POST".into(), "PUT".into(), "DELETE".into()],
            allowed_headers: Some(vec!["*".into()]),
            expose_headers: Some(vec!["ETag".into(), "x-amz-request-id".into()]),
            max_age_seconds: Some(3600),
        },
    ];

    client
        .put_bucket_cors(&bucket, rules)
        .await
        .expect("put multiple cors");

    tokio::time::sleep(Duration::from_millis(500)).await;

    let fetched = client.get_bucket_cors(&bucket).await.expect("get cors");
    assert_eq!(fetched.len(), 2, "Should have 2 CORS rules");

    // Cleanup
    client.delete_bucket_cors(&bucket).await.expect("cleanup");
}

// ═══════════════════════════════════════════════════════════════════════════
// Lifecycle Management
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn test_lifecycle_crud() {
    let client = get_client();
    let bucket = test_bucket();

    // Delete existing rules (clean state)
    let _ = client.delete_lifecycle_rules(&bucket).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // GET — should be empty
    let rules = client
        .get_lifecycle_rules(&bucket)
        .await
        .expect("get_lifecycle_rules");
    assert!(rules.is_empty(), "Lifecycle should be empty initially");

    // PUT
    let rule = LifecycleRule {
        id: "test-expire-tmp".into(),
        prefix: Some("tmp/".into()),
        enabled: true,
        expiration_days: Some(7),
        noncurrent_version_expiration_days: None,
        transition_days: None,
        transition_storage_class: None,
        tags: None,
        object_size_greater_than: None,
        object_size_less_than: None,
    };

    client
        .put_lifecycle_rules(&bucket, vec![rule])
        .await
        .expect("put_lifecycle_rules failed");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // GET — verify
    let rules = client
        .get_lifecycle_rules(&bucket)
        .await
        .expect("get_lifecycle_rules after put");
    assert_eq!(rules.len(), 1, "Should have 1 lifecycle rule");

    let r = &rules[0];
    assert_eq!(r.id, "test-expire-tmp");
    assert_eq!(r.prefix, Some("tmp/".into()));
    assert!(r.enabled);
    assert_eq!(r.expiration_days, Some(7));

    // DELETE
    client
        .delete_lifecycle_rules(&bucket)
        .await
        .expect("delete_lifecycle_rules");

    tokio::time::sleep(Duration::from_millis(500)).await;

    let rules = client
        .get_lifecycle_rules(&bucket)
        .await
        .expect("get after delete");
    assert!(rules.is_empty(), "Lifecycle should be empty after delete");
}

#[tokio::test]
#[ignore]
async fn test_lifecycle_with_tags() {
    let client = get_client();
    let bucket = test_bucket();

    let _ = client.delete_lifecycle_rules(&bucket).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let rule = LifecycleRule {
        id: "tagged-rule".into(),
        prefix: Some("logs/".into()),
        enabled: true,
        expiration_days: Some(30),
        noncurrent_version_expiration_days: Some(7),
        transition_days: None,
        transition_storage_class: None,
        tags: Some(vec![LifecycleTag {
            key: "env".into(),
            value: "test".into(),
        }]),
        object_size_greater_than: None,
        object_size_less_than: None,
    };

    client
        .put_lifecycle_rules(&bucket, vec![rule])
        .await
        .expect("put lifecycle with tags");

    tokio::time::sleep(Duration::from_millis(500)).await;

    let rules = client.get_lifecycle_rules(&bucket).await.expect("get");
    assert_eq!(rules.len(), 1);
    assert!(rules[0].tags.is_some());
    let tags = rules[0].tags.as_ref().unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].key, "env");
    assert_eq!(tags[0].value, "test");

    // Cleanup
    client
        .delete_lifecycle_rules(&bucket)
        .await
        .expect("cleanup");
}

// ═══════════════════════════════════════════════════════════════════════════
// Bucket Policy
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn test_policy_crud() {
    let client = get_client();
    let bucket = test_bucket();

    // Delete existing policy (clean state)
    let _ = client.delete_bucket_policy(&bucket).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // GET — should be empty
    let policy = client
        .get_bucket_policy(&bucket)
        .await
        .expect("get_bucket_policy");
    assert!(
        policy.policy_json.is_empty(),
        "Policy should be empty initially"
    );

    // PUT
    let policy_doc = serde_json::json!({
        "Version": "2012-10-17",
        "Statement": [{
            "Sid": "TestPublicRead",
            "Effect": "Allow",
            "Principal": "*",
            "Action": "s3:GetObject",
            "Resource": format!("arn:aws:s3:::{}/__test_policy__/*", bucket)
        }]
    });

    client
        .put_bucket_policy(
            &bucket,
            BucketPolicy {
                policy_json: policy_doc.to_string(),
            },
        )
        .await
        .expect("put_bucket_policy failed");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // GET — verify
    let policy = client
        .get_bucket_policy(&bucket)
        .await
        .expect("get after put");
    assert!(!policy.policy_json.is_empty(), "Policy should not be empty");

    let parsed: serde_json::Value =
        serde_json::from_str(&policy.policy_json).expect("valid JSON");
    // OBS may normalize the policy version; accept either
    let version = parsed["Version"].as_str().unwrap_or("");
    assert!(
        version == "2012-10-17" || version == "2008-10-17",
        "Unexpected policy version: {}",
        version
    );

    // DELETE
    client
        .delete_bucket_policy(&bucket)
        .await
        .expect("delete_bucket_policy");

    tokio::time::sleep(Duration::from_millis(500)).await;

    let policy = client
        .get_bucket_policy(&bucket)
        .await
        .expect("get after delete");
    assert!(
        policy.policy_json.is_empty(),
        "Policy should be empty after delete"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// List objects edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn test_list_objects_empty_prefix() {
    let client = get_client();
    let bucket = test_bucket();

    let result = client
        .list_objects(
            &bucket,
            ListOptions {
                prefix: Some("__nonexistent_prefix_xyz__/".into()),
                ..Default::default()
            },
        )
        .await
        .expect("list_objects");
    assert!(result.objects.is_empty(), "No objects with fake prefix");
    assert!(result.common_prefixes.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_list_objects_special_characters() {
    let client = get_client();
    let bucket = test_bucket();
    let prefix = test_prefix();
    let key = format!("{}special chars (1) [2] {{3}}.txt", prefix);

    client
        .upload_object(&bucket, &key, b"special".to_vec().into(), None)
        .await
        .expect("upload special chars");

    let result = client
        .list_objects(
            &bucket,
            ListOptions {
                prefix: Some(prefix.clone()),
                ..Default::default()
            },
        )
        .await
        .expect("list");
    assert_eq!(result.objects.len(), 1);
    assert_eq!(result.objects[0].key, key);

    client.delete_object(&bucket, &key).await.expect("cleanup");
}

// ═══════════════════════════════════════════════════════════════════════════
// Delete CORS idempotency
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn test_delete_cors_idempotent() {
    let client = get_client();
    let bucket = test_bucket();

    // Delete twice — should not error
    let _ = client.delete_bucket_cors(&bucket).await;
    let result = client.delete_bucket_cors(&bucket).await;
    assert!(result.is_ok(), "Deleting CORS twice should be idempotent");
}

#[tokio::test]
#[ignore]
async fn test_delete_lifecycle_idempotent() {
    let client = get_client();
    let bucket = test_bucket();

    let _ = client.delete_lifecycle_rules(&bucket).await;
    let result = client.delete_lifecycle_rules(&bucket).await;
    assert!(
        result.is_ok(),
        "Deleting lifecycle twice should be idempotent"
    );
}

#[tokio::test]
#[ignore]
async fn test_delete_policy_idempotent() {
    let client = get_client();
    let bucket = test_bucket();

    let _ = client.delete_bucket_policy(&bucket).await;
    let result = client.delete_bucket_policy(&bucket).await;
    assert!(
        result.is_ok(),
        "Deleting policy twice should be idempotent"
    );
}
