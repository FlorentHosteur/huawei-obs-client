//! CORS management example.
//!
//! Run with:
//! ```bash
//! OBS_ACCESS_KEY=ak OBS_SECRET_KEY=sk OBS_ENDPOINT=https://obs.example.com \
//!   cargo run --example cors -- my-bucket
//! ```

use huawei_obs_client::{CorsRule, ObsClient};

#[tokio::main]
async fn main() -> huawei_obs_client::Result<()> {
    let ak = std::env::var("OBS_ACCESS_KEY").expect("OBS_ACCESS_KEY required");
    let sk = std::env::var("OBS_SECRET_KEY").expect("OBS_SECRET_KEY required");
    let endpoint = std::env::var("OBS_ENDPOINT").expect("OBS_ENDPOINT required");

    let bucket = std::env::args()
        .nth(1)
        .expect("Usage: cors <bucket-name>");

    let client = ObsClient::builder()
        .access_key(ak)
        .secret_key(sk)
        .endpoint(endpoint)
        .build()?;

    // Show current CORS
    println!("=== Current CORS Rules ===");
    let rules = client.get_bucket_cors(&bucket).await?;
    if rules.is_empty() {
        println!("  (none)");
    }
    for rule in &rules {
        println!(
            "  [{}] origins={:?} methods={:?} max_age={}s",
            rule.id,
            rule.allowed_origins,
            rule.allowed_methods,
            rule.max_age_seconds.unwrap_or(0)
        );
    }

    // Set a permissive CORS rule
    println!("\n=== Setting permissive CORS ===");
    let cors = CorsRule {
        id: "allow-all".into(),
        allowed_origins: vec!["*".into()],
        allowed_methods: vec!["GET".into(), "PUT".into(), "POST".into(), "DELETE".into()],
        allowed_headers: Some(vec!["*".into()]),
        expose_headers: Some(vec!["ETag".into()]),
        max_age_seconds: Some(3000),
    };

    client.put_bucket_cors(&bucket, vec![cors]).await?;
    println!("  Done!");

    // Verify
    println!("\n=== Updated CORS Rules ===");
    for rule in client.get_bucket_cors(&bucket).await? {
        println!(
            "  [{}] origins={:?} methods={:?} headers={:?} expose={:?} max_age={}s",
            rule.id,
            rule.allowed_origins,
            rule.allowed_methods,
            rule.allowed_headers,
            rule.expose_headers,
            rule.max_age_seconds.unwrap_or(0)
        );
    }

    Ok(())
}
