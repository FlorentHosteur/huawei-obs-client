//! # huawei-obs-client
//!
//! A Rust client library for **Huawei OBS** (Object Storage Service), which is
//! S3-compatible. Provides a clean, async API for bucket and object operations,
//! lifecycle rules, CORS, bucket policies, versioning, and Object Lock (WORM).
//!
//! ## Quick Start
//!
//! ```no_run
//! use huawei_obs_client::ObsClient;
//!
//! #[tokio::main]
//! async fn main() -> huawei_obs_client::Result<()> {
//!     // Build a client
//!     let client = ObsClient::builder()
//!         .access_key("your-access-key")
//!         .secret_key("your-secret-key")
//!         .endpoint("https://obs.ap-southeast-1.myhuaweicloud.com")
//!         .region("ap-southeast-1")
//!         .build()?;
//!
//!     // List buckets
//!     let buckets = client.list_buckets().await?;
//!     for b in &buckets {
//!         println!("  {}", b.name);
//!     }
//!
//!     // Upload
//!     client
//!         .upload_object("my-bucket", "greeting.txt", "Hello!".into(), None)
//!         .await?;
//!
//!     // Download
//!     let data = client.download_object("my-bucket", "greeting.txt").await?;
//!     println!("{}", String::from_utf8_lossy(&data));
//!
//!     // Delete
//!     client.delete_object("my-bucket", "greeting.txt").await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! | Category | Operations |
//! |----------|-----------|
//! | **Buckets** | list, create, delete, exists |
//! | **Objects** | upload (bytes/stream), download, delete, batch delete, copy, exists, metadata |
//! | **Presigned URLs** | GET, PUT |
//! | **Lifecycle** | get, put, delete rules |
//! | **CORS** | get, put, delete rules (auto Content-MD5) |
//! | **Policy** | get, put, delete bucket policies |
//! | **Versioning** | get/set bucket state, list versions, download/delete/copy a version |
//! | **Object Lock** | get/set configuration, get/set object retention |
//!
//! ## Error Handling
//!
//! All operations return `Result<T, ObsError>`. Use [`ObsError::user_message()`]
//! for end-user-friendly messages:
//!
//! ```no_run
//! # use huawei_obs_client::{ObsClient, ObsError};
//! # async fn example(client: &ObsClient) {
//! match client.delete_bucket("test").await {
//!     Ok(_) => println!("Deleted!"),
//!     Err(ObsError::BucketNotEmpty(name)) => {
//!         eprintln!("Bucket '{}' is not empty", name);
//!     }
//!     Err(e) => eprintln!("{}", e.user_message()),
//! }
//! # }
//! ```

pub mod client;
pub mod error;
pub mod types;

// Re-export the main public API at crate root.
pub use client::{ObsClient, ObsClientBuilder};
pub use error::{ObsError, Result};
pub use types::*;
