//! Lifecycle rules management example.
//!
//! Run with:
//! ```bash
//! OBS_ACCESS_KEY=ak OBS_SECRET_KEY=sk OBS_ENDPOINT=https://obs.example.com \
//!   cargo run --example lifecycle -- my-bucket
//! ```

use huawei_obs_client::{LifecycleRule, ObsClient};

#[tokio::main]
async fn main() -> huawei_obs_client::Result<()> {
    let ak = std::env::var("OBS_ACCESS_KEY").expect("OBS_ACCESS_KEY required");
    let sk = std::env::var("OBS_SECRET_KEY").expect("OBS_SECRET_KEY required");
    let endpoint = std::env::var("OBS_ENDPOINT").expect("OBS_ENDPOINT required");

    let bucket = std::env::args()
        .nth(1)
        .expect("Usage: lifecycle <bucket-name>");

    let client = ObsClient::builder()
        .access_key(ak)
        .secret_key(sk)
        .endpoint(endpoint)
        .build()?;

    // Show current rules
    println!("=== Current Lifecycle Rules ===");
    let rules = client.get_lifecycle_rules(&bucket).await?;
    if rules.is_empty() {
        println!("  (none)");
    }
    for rule in &rules {
        println!(
            "  [{}] prefix={:?} enabled={} expire={}d",
            rule.id,
            rule.prefix,
            rule.enabled,
            rule.expiration_days.unwrap_or(0)
        );
    }

    // Add a rule: expire objects under tmp/ after 7 days
    println!("\n=== Adding rule: expire tmp/ after 7 days ===");
    let new_rule = LifecycleRule {
        id: "expire-tmp".into(),
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

    let mut all_rules = client.get_lifecycle_rules(&bucket).await?;
    all_rules.push(new_rule);
    client.put_lifecycle_rules(&bucket, all_rules).await?;
    println!("  Done!");

    // Verify
    println!("\n=== Updated Rules ===");
    for rule in client.get_lifecycle_rules(&bucket).await? {
        println!(
            "  [{}] prefix={:?} enabled={} expire={}d",
            rule.id,
            rule.prefix,
            rule.enabled,
            rule.expiration_days.unwrap_or(0)
        );
    }

    Ok(())
}
