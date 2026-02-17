//! Basic usage example for huawei-obs-client.
//!
//! Run with:
//! ```bash
//! OBS_ACCESS_KEY=your-ak OBS_SECRET_KEY=your-sk OBS_ENDPOINT=https://obs.example.com cargo run --example basic
//! ```

use huawei_obs_client::{ListOptions, ObsClient, UploadOptions};

#[tokio::main]
async fn main() -> huawei_obs_client::Result<()> {
    let ak = std::env::var("OBS_ACCESS_KEY").expect("OBS_ACCESS_KEY env var required");
    let sk = std::env::var("OBS_SECRET_KEY").expect("OBS_SECRET_KEY env var required");
    let endpoint = std::env::var("OBS_ENDPOINT").expect("OBS_ENDPOINT env var required");
    let region = std::env::var("OBS_REGION").unwrap_or_else(|_| "us-east-1".to_string());

    let client = ObsClient::builder()
        .access_key(ak)
        .secret_key(sk)
        .endpoint(endpoint)
        .region(region)
        .build()?;

    // ── List Buckets ─────────────────────────────────────────────────────
    println!("=== Buckets ===");
    let buckets = client.list_buckets().await?;
    for b in &buckets {
        println!("  {}", b.name);
    }

    if buckets.is_empty() {
        println!("  (no buckets found)");
        return Ok(());
    }

    let bucket = &buckets[0].name;
    println!("\nUsing bucket: {}\n", bucket);

    // ── Upload ───────────────────────────────────────────────────────────
    println!("=== Upload ===");
    let content = b"Hello from huawei-obs-client!";
    client
        .upload_object(
            bucket,
            "test/hello.txt",
            content.to_vec().into(),
            Some(UploadOptions {
                content_type: Some("text/plain".into()),
                ..Default::default()
            }),
        )
        .await?;
    println!("  Uploaded test/hello.txt");

    // ── List Objects ─────────────────────────────────────────────────────
    println!("\n=== Objects in test/ ===");
    let result = client
        .list_objects(
            bucket,
            ListOptions {
                prefix: Some("test/".into()),
                ..Default::default()
            },
        )
        .await?;
    for obj in &result.objects {
        println!("  {} ({} bytes)", obj.key, obj.size);
    }

    // ── Download ─────────────────────────────────────────────────────────
    println!("\n=== Download ===");
    let data = client.download_object(bucket, "test/hello.txt").await?;
    println!("  Content: {}", String::from_utf8_lossy(&data));

    // ── Presigned URL ────────────────────────────────────────────────────
    println!("\n=== Presigned URL ===");
    let url = client.presign_get(bucket, "test/hello.txt", 3600).await?;
    println!("  GET URL (1h): {}", url);

    // ── Metadata ─────────────────────────────────────────────────────────
    println!("\n=== Metadata ===");
    let meta = client
        .get_object_metadata(bucket, "test/hello.txt")
        .await?;
    println!("  Content-Type: {:?}", meta.content_type);
    println!("  Size: {} bytes", meta.content_length);
    println!("  ETag: {:?}", meta.etag);

    // ── Cleanup ──────────────────────────────────────────────────────────
    println!("\n=== Cleanup ===");
    client.delete_object(bucket, "test/hello.txt").await?;
    println!("  Deleted test/hello.txt");

    println!("\nDone!");
    Ok(())
}
