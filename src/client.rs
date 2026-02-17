//! Huawei OBS client implementation.

use crate::error::{ObsError, Result};
use crate::types::*;
use bytes::Bytes;
use s3::{creds::Credentials, error::S3Error, Bucket, Region};
use std::collections::HashMap;
use std::time::SystemTime;
use tokio::io::AsyncRead;

/// Client for interacting with Huawei OBS (Object Storage Service).
///
/// Uses both the `rust-s3` crate and the official AWS SDK internally. The `rust-s3`
/// crate handles basic S3 operations and properly sends the `Content-MD5` header
/// required by Huawei OBS. The AWS SDK is used for advanced operations like
/// lifecycle rules, CORS, and bucket policies where it provides richer type support.
///
/// # Example
///
/// ```no_run
/// use huawei_obs_client::ObsClient;
///
/// # async fn example() -> huawei_obs_client::Result<()> {
/// let client = ObsClient::builder()
///     .access_key("your-ak")
///     .secret_key("your-sk")
///     .endpoint("https://obs.ap-southeast-1.myhuaweicloud.com")
///     .region("ap-southeast-1")
///     .build()?;
///
/// // List all buckets
/// let buckets = client.list_buckets().await?;
/// for b in &buckets {
///     println!("{}", b.name);
/// }
///
/// // Upload an object
/// client.upload_object("my-bucket", "hello.txt", "Hello, world!".into(), None).await?;
///
/// // Download it back
/// let data = client.download_object("my-bucket", "hello.txt").await?;
/// println!("{}", String::from_utf8_lossy(&data));
/// # Ok(())
/// # }
/// ```
pub struct ObsClient {
    #[allow(dead_code)]
    bucket: Box<Bucket>,
    credentials: Credentials,
    region: Region,
    access_key: String,
    secret_key: String,
    endpoint: String,
    region_name: String,
}

/// Builder for constructing an [`ObsClient`].
///
/// # Example
///
/// ```no_run
/// # use huawei_obs_client::ObsClient;
/// let client = ObsClient::builder()
///     .access_key("AK")
///     .secret_key("SK")
///     .endpoint("https://obs.ap-southeast-1.myhuaweicloud.com")
///     .region("ap-southeast-1")
///     .build()
///     .expect("valid config");
/// ```
pub struct ObsClientBuilder {
    access_key: Option<String>,
    secret_key: Option<String>,
    endpoint: Option<String>,
    region: Option<String>,
}

impl ObsClientBuilder {
    /// Set the access key.
    pub fn access_key(mut self, ak: impl Into<String>) -> Self {
        self.access_key = Some(ak.into());
        self
    }

    /// Set the secret key.
    pub fn secret_key(mut self, sk: impl Into<String>) -> Self {
        self.secret_key = Some(sk.into());
        self
    }

    /// Set the OBS endpoint URL (e.g. `https://obs.ap-southeast-1.myhuaweicloud.com`).
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set the region identifier (e.g. `ap-southeast-1`).
    /// Defaults to `"us-east-1"` if not set.
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Build the [`ObsClient`].
    ///
    /// # Errors
    ///
    /// Returns [`ObsError::InvalidConfig`] if access_key, secret_key, or endpoint is missing.
    pub fn build(self) -> Result<ObsClient> {
        let access_key = self
            .access_key
            .ok_or_else(|| ObsError::InvalidConfig("access_key is required".into()))?;
        let secret_key = self
            .secret_key
            .ok_or_else(|| ObsError::InvalidConfig("secret_key is required".into()))?;
        let endpoint = self
            .endpoint
            .ok_or_else(|| ObsError::InvalidConfig("endpoint is required".into()))?;
        let region = self.region.unwrap_or_else(|| "us-east-1".to_string());

        ObsClient::new(access_key, secret_key, endpoint, region)
    }
}

impl ObsClient {
    /// Create a builder for configuring an [`ObsClient`].
    pub fn builder() -> ObsClientBuilder {
        ObsClientBuilder {
            access_key: None,
            secret_key: None,
            endpoint: None,
            region: None,
        }
    }

    /// Create a new OBS client directly.
    ///
    /// Prefer [`ObsClient::builder()`] for a more readable API.
    pub fn new(
        access_key: String,
        secret_key: String,
        endpoint: String,
        region: String,
    ) -> Result<Self> {
        let credentials = Credentials::new(
            Some(&access_key),
            Some(&secret_key),
            None,
            None,
            None,
        )
        .map_err(|e| ObsError::S3Error(e.to_string()))?;

        let s3_region = Region::Custom {
            region: region.clone(),
            endpoint: endpoint.clone(),
        };

        let bucket = Bucket::new("dummy", s3_region.clone(), credentials.clone())
            .map_err(|e| ObsError::S3Error(e.to_string()))?
            .with_path_style();

        Ok(Self {
            bucket,
            credentials,
            region: s3_region,
            access_key,
            secret_key,
            endpoint,
            region_name: region,
        })
    }

    /// Create an AWS SDK client for operations not supported by rust-s3.
    async fn get_aws_client(&self) -> Result<aws_sdk_s3::Client> {
        let credentials = aws_sdk_s3::config::Credentials::new(
            &self.access_key,
            &self.secret_key,
            None,
            None,
            "static",
        );

        let region = aws_sdk_s3::config::Region::new(self.region_name.clone());

        let config = aws_sdk_s3::config::Builder::new()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(region)
            .endpoint_url(&self.endpoint)
            .force_path_style(true)
            .build();

        Ok(aws_sdk_s3::Client::from_conf(config))
    }

    /// Get a rust-s3 bucket handle for the given bucket name.
    fn get_bucket(&self, name: &str) -> Result<Box<Bucket>> {
        Bucket::new(name, self.region.clone(), self.credentials.clone())
            .map_err(|e| ObsError::S3Error(e.to_string()))
            .map(|b| b.with_path_style())
    }

    /// Convert a rust-s3 error into an [`ObsError`].
    fn convert_s3_error(err: S3Error) -> ObsError {
        let err_str = err.to_string();
        let err_lower = err_str.to_lowercase();

        // Serde/parsing errors often indicate access denied
        if err_lower.contains("serde")
            || err_lower.contains("missing field")
            || err_lower.contains("deserialize")
        {
            return ObsError::PermissionDenied(String::new());
        }

        match &err {
            S3Error::HttpFailWithBody(status, body) => {
                let body_lower = body.to_lowercase();
                match *status {
                    404 => {
                        if body_lower.contains("nosuchbucket") {
                            ObsError::BucketNotFound(String::new())
                        } else {
                            ObsError::NotFound(String::new())
                        }
                    }
                    403 => ObsError::PermissionDenied(String::new()),
                    401 => ObsError::InvalidCredentials,
                    409 => {
                        if body_lower.contains("bucketnotempty") {
                            ObsError::BucketNotEmpty(String::new())
                        } else if body_lower.contains("bucketalreadyexists") {
                            ObsError::BucketAlreadyExists(String::new())
                        } else {
                            ObsError::S3Error(body.clone())
                        }
                    }
                    500..=599 => ObsError::S3Error(format!("Server error ({})", status)),
                    _ => ObsError::S3Error(body.clone()),
                }
            }
            S3Error::Credentials(_) => ObsError::InvalidCredentials,
            S3Error::Io(e) => ObsError::Network(e.to_string()),
            S3Error::Reqwest(e) => {
                let msg = e.to_string().to_lowercase();
                if msg.contains("timeout") {
                    ObsError::Timeout(String::new())
                } else if msg.contains("connect") {
                    ObsError::ConnectionFailed(String::new())
                } else {
                    ObsError::Network(e.to_string())
                }
            }
            _ => {
                if err_lower.contains("access") || err_lower.contains("denied") {
                    ObsError::PermissionDenied(String::new())
                } else if err_lower.contains("not found") || err_lower.contains("nosuch") {
                    ObsError::NotFound(String::new())
                } else {
                    ObsError::Other(err_str)
                }
            }
        }
    }

    /// Convert an AWS SDK error (Debug formatted) into an [`ObsError`].
    fn convert_aws_error<E: std::fmt::Debug>(err: E) -> ObsError {
        let msg = format!("{:?}", err);
        let lower = msg.to_lowercase();

        if lower.contains("accessdenied") || lower.contains("access denied") {
            return ObsError::PermissionDenied(String::new());
        }
        if lower.contains("invalidaccesskeyid") || lower.contains("signaturemismatch") {
            return ObsError::InvalidCredentials;
        }
        if lower.contains("nosuchbucket") {
            return ObsError::BucketNotFound(String::new());
        }
        if lower.contains("bucketalreadyexists") || lower.contains("bucketalreadyownedby") {
            return ObsError::BucketAlreadyExists(String::new());
        }
        if lower.contains("bucketnotempty") || lower.contains("not empty") {
            return ObsError::BucketNotEmpty(String::new());
        }
        if lower.contains("nosuchkey") || lower.contains("notfound") || lower.contains("404") {
            return ObsError::NotFound(String::new());
        }
        if lower.contains("timeout") || lower.contains("timed out") {
            return ObsError::Timeout(String::new());
        }
        if lower.contains("connection") {
            return ObsError::ConnectionFailed(String::new());
        }

        ObsError::AwsSdk(msg)
    }
}

// ─── Bucket Operations ───────────────────────────────────────────────────────

impl ObsClient {
    /// List all buckets accessible with the current credentials.
    pub async fn list_buckets(&self) -> Result<Vec<BucketInfo>> {
        let client = self.get_aws_client().await?;

        let response = client
            .list_buckets()
            .send()
            .await
            .map_err(|e| ObsError::AwsSdk(e.to_string()))?;

        let buckets = response
            .buckets()
            .iter()
            .map(|bucket| BucketInfo {
                name: bucket.name().unwrap_or("").to_string(),
                creation_date: bucket.creation_date().and_then(|dt| {
                    SystemTime::UNIX_EPOCH.checked_add(
                        std::time::Duration::from_secs(dt.secs() as u64)
                            + std::time::Duration::from_nanos(dt.subsec_nanos() as u64),
                    )
                }),
                region: None,
            })
            .collect();

        Ok(buckets)
    }

    /// Create a new bucket.
    pub async fn create_bucket(&self, name: &str) -> Result<()> {
        let client = self.get_aws_client().await?;

        client
            .create_bucket()
            .bucket(name)
            .send()
            .await
            .map_err(Self::convert_aws_error)?;

        Ok(())
    }

    /// Delete an empty bucket.
    pub async fn delete_bucket(&self, name: &str) -> Result<()> {
        let client = self.get_aws_client().await?;

        client
            .delete_bucket()
            .bucket(name)
            .send()
            .await
            .map_err(|e| {
                let err_str = format!("{:?}", e).to_lowercase();
                if err_str.contains("not empty")
                    || err_str.contains("notempty")
                    || err_str.contains("bucketnotempty")
                {
                    ObsError::BucketNotEmpty(name.to_string())
                } else if err_str.contains("not found")
                    || err_str.contains("nosuchbucket")
                    || err_str.contains("404")
                {
                    ObsError::BucketNotFound(name.to_string())
                } else {
                    ObsError::AwsSdk(format!("{:?}", e))
                }
            })?;

        Ok(())
    }

    /// Check whether a bucket exists.
    pub async fn bucket_exists(&self, name: &str) -> Result<bool> {
        let bucket = self.get_bucket(name)?;
        bucket.exists().await.map_err(Self::convert_s3_error)
    }
}

// ─── Object Operations ───────────────────────────────────────────────────────

impl ObsClient {
    /// Upload an object from an in-memory buffer.
    pub async fn upload_object(
        &self,
        bucket: &str,
        key: &str,
        data: Bytes,
        options: Option<UploadOptions>,
    ) -> Result<()> {
        let bucket = self.get_bucket(bucket)?;

        let content_type = options
            .as_ref()
            .and_then(|o| o.content_type.as_deref())
            .unwrap_or("application/octet-stream");

        bucket
            .put_object_with_content_type(key, &data, content_type)
            .await
            .map_err(Self::convert_s3_error)?;

        Ok(())
    }

    /// Upload an object from an async reader (streaming).
    pub async fn upload_object_stream(
        &self,
        bucket: &str,
        key: &str,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        content_type: Option<&str>,
    ) -> Result<()> {
        let bucket = self.get_bucket(bucket)?;
        let ct = content_type.unwrap_or("application/octet-stream");

        bucket
            .put_object_stream_with_content_type(&mut reader, key, ct)
            .await
            .map_err(Self::convert_s3_error)?;

        Ok(())
    }

    /// Download an object and return its contents as bytes.
    pub async fn download_object(&self, bucket: &str, key: &str) -> Result<Bytes> {
        let bucket = self.get_bucket(bucket)?;

        let response = bucket
            .get_object(key)
            .await
            .map_err(Self::convert_s3_error)?;

        Ok(Bytes::from(response.bytes().to_vec()))
    }

    /// Delete a single object.
    pub async fn delete_object(&self, bucket: &str, key: &str) -> Result<()> {
        let bucket = self.get_bucket(bucket)?;

        bucket
            .delete_object(key)
            .await
            .map_err(Self::convert_s3_error)?;

        Ok(())
    }

    /// Delete multiple objects. Returns the keys that were successfully deleted.
    pub async fn delete_objects(&self, bucket: &str, keys: Vec<String>) -> Result<Vec<String>> {
        let bucket = self.get_bucket(bucket)?;
        let mut deleted = Vec::new();

        for key in keys {
            match bucket.delete_object(&key).await {
                Ok(_) => deleted.push(key),
                Err(e) => {
                    let err = Self::convert_s3_error(e);
                    eprintln!("Failed to delete '{}': {}", key, err.user_message());
                }
            }
        }

        Ok(deleted)
    }

    /// Check whether an object exists.
    pub async fn object_exists(&self, bucket: &str, key: &str) -> Result<bool> {
        let bucket = self.get_bucket(bucket)?;

        match bucket.head_object(key).await {
            Ok(_) => Ok(true),
            Err(S3Error::HttpFailWithBody(404, _)) => Ok(false),
            Err(e) => Err(Self::convert_s3_error(e)),
        }
    }

    /// Get metadata for an object (content type, size, ETag, etc.).
    pub async fn get_object_metadata(&self, bucket: &str, key: &str) -> Result<ObjectMetadata> {
        let bucket = self.get_bucket(bucket)?;

        let (head, _status) = bucket
            .head_object(key)
            .await
            .map_err(Self::convert_s3_error)?;

        Ok(ObjectMetadata {
            content_type: head.content_type,
            content_length: head.content_length.unwrap_or(0) as u64,
            etag: head.e_tag,
            last_modified: None,
            metadata: HashMap::new(),
        })
    }

    /// List objects in a bucket with optional filtering.
    pub async fn list_objects(&self, bucket_name: &str, options: ListOptions) -> Result<ListResult> {
        let bucket = self.get_bucket(bucket_name)?;

        let prefix = options.prefix.unwrap_or_default();
        let delimiter = options.delimiter;

        let results = bucket
            .list(prefix, delimiter)
            .await
            .map_err(Self::convert_s3_error)?;

        let mut all_objects = Vec::new();
        let mut all_prefixes = Vec::new();

        for list in results {
            for obj in list.contents {
                all_objects.push(ObjectInfo {
                    key: obj.key.clone(),
                    size: obj.size,
                    last_modified: None,
                    etag: obj.e_tag.clone(),
                    storage_class: obj.storage_class.clone(),
                });
            }

            if let Some(cp_vec) = list.common_prefixes {
                for cp in cp_vec {
                    all_prefixes.push(cp.prefix);
                }
            }
        }

        Ok(ListResult {
            objects: all_objects,
            common_prefixes: all_prefixes,
            is_truncated: false,
            next_continuation_token: None,
        })
    }

    /// Copy an object within the same bucket or across buckets.
    pub async fn copy_object(
        &self,
        source_bucket: &str,
        source_key: &str,
        dest_bucket: &str,
        dest_key: &str,
    ) -> Result<()> {
        let bucket = self.get_bucket(dest_bucket)?;

        if source_bucket == dest_bucket {
            bucket
                .copy_object_internal(source_key, dest_key)
                .await
                .map_err(Self::convert_s3_error)?;
        } else {
            let source = self.get_bucket(source_bucket)?;
            let data = source
                .get_object(source_key)
                .await
                .map_err(Self::convert_s3_error)?;

            bucket
                .put_object(dest_key, data.bytes())
                .await
                .map_err(Self::convert_s3_error)?;
        }

        Ok(())
    }

    /// Generate a presigned URL for downloading an object.
    pub async fn presign_get(
        &self,
        bucket: &str,
        key: &str,
        expires_in_secs: u64,
    ) -> Result<String> {
        let bucket = self.get_bucket(bucket)?;

        bucket
            .presign_get(key, expires_in_secs as u32, None)
            .await
            .map_err(Self::convert_s3_error)
    }

    /// Generate a presigned URL for uploading an object.
    pub async fn presign_put(
        &self,
        bucket: &str,
        key: &str,
        expires_in_secs: u64,
    ) -> Result<String> {
        let bucket = self.get_bucket(bucket)?;

        bucket
            .presign_put(key, expires_in_secs as u32, None, None)
            .await
            .map_err(Self::convert_s3_error)
    }
}

// ─── Lifecycle Management ────────────────────────────────────────────────────

impl ObsClient {
    /// Get lifecycle rules for a bucket. Returns an empty vec if none are configured.
    pub async fn get_lifecycle_rules(&self, bucket_name: &str) -> Result<Vec<LifecycleRule>> {
        use aws_sdk_s3::types::ExpirationStatus;

        let client = self.get_aws_client().await?;

        let result = client
            .get_bucket_lifecycle_configuration()
            .bucket(bucket_name)
            .send()
            .await;

        match result {
            Ok(output) => {
                let rules = output
                    .rules()
                    .iter()
                    .map(|rule| {
                        let mut prefix: Option<String> = None;
                        let mut tags: Option<Vec<LifecycleTag>> = None;
                        let mut object_size_greater_than: Option<i64> = None;
                        let mut object_size_less_than: Option<i64> = None;

                        if let Some(filter) = rule.filter() {
                            if let Some(p) = filter.prefix() {
                                if !p.is_empty() {
                                    prefix = Some(p.to_string());
                                }
                            }
                            if let Some(tag) = filter.tag() {
                                tags = Some(vec![LifecycleTag {
                                    key: tag.key().to_string(),
                                    value: tag.value().to_string(),
                                }]);
                            }
                            if let Some(and_op) = filter.and() {
                                if let Some(p) = and_op.prefix() {
                                    if !p.is_empty() {
                                        prefix = Some(p.to_string());
                                    }
                                }
                                let and_tags: Vec<LifecycleTag> = and_op
                                    .tags()
                                    .iter()
                                    .map(|t| LifecycleTag {
                                        key: t.key().to_string(),
                                        value: t.value().to_string(),
                                    })
                                    .collect();
                                if !and_tags.is_empty() {
                                    tags = Some(and_tags);
                                }
                                if let Some(size) = and_op.object_size_greater_than() {
                                    object_size_greater_than = Some(size);
                                }
                                if let Some(size) = and_op.object_size_less_than() {
                                    object_size_less_than = Some(size);
                                }
                            }
                            if let Some(size) = filter.object_size_greater_than() {
                                object_size_greater_than = Some(size);
                            }
                            if let Some(size) = filter.object_size_less_than() {
                                object_size_less_than = Some(size);
                            }
                        }

                        LifecycleRule {
                            id: rule.id().unwrap_or_default().to_string(),
                            prefix,
                            enabled: rule.status() == &ExpirationStatus::Enabled,
                            expiration_days: rule
                                .expiration()
                                .and_then(|e| e.days()),
                            noncurrent_version_expiration_days: rule
                                .noncurrent_version_expiration()
                                .and_then(|n| n.noncurrent_days()),
                            transition_days: None,
                            transition_storage_class: None,
                            tags,
                            object_size_greater_than,
                            object_size_less_than,
                        }
                    })
                    .collect();
                Ok(rules)
            }
            Err(e) => {
                let err_str = format!("{:?}", e);
                if err_str.contains("NoSuchLifecycleConfiguration")
                    || err_str.contains("404")
                    || err_str.contains("NoSuchConfiguration")
                    || err_str.contains("does not exist")
                    || err_str.contains("missing field")
                {
                    Ok(vec![])
                } else {
                    Err(ObsError::S3Error(format!(
                        "Failed to get lifecycle rules: {}",
                        err_str
                    )))
                }
            }
        }
    }

    /// Set lifecycle rules for a bucket (replaces all existing rules).
    pub async fn put_lifecycle_rules(
        &self,
        bucket_name: &str,
        rules: Vec<LifecycleRule>,
    ) -> Result<()> {
        use aws_sdk_s3::types::{
            BucketLifecycleConfiguration, ExpirationStatus,
            LifecycleExpiration, LifecycleRule as AwsLifecycleRule,
            LifecycleRuleAndOperator, LifecycleRuleFilter,
            NoncurrentVersionExpiration, Tag as AwsTag,
        };

        let client = self.get_aws_client().await?;

        let aws_rules: Vec<AwsLifecycleRule> = rules
            .into_iter()
            .map(|rule| {
                let status = if rule.enabled {
                    ExpirationStatus::Enabled
                } else {
                    ExpirationStatus::Disabled
                };

                let has_tags = rule.tags.as_ref().is_some_and(|t| !t.is_empty());
                let has_size_filters = rule.object_size_greater_than.is_some()
                    || rule.object_size_less_than.is_some();
                let has_prefix = rule.prefix.as_ref().is_some_and(|p| !p.is_empty());
                let needs_and =
                    (has_prefix as i32) + (has_tags as i32) + (has_size_filters as i32) > 1
                        || has_tags
                        || has_size_filters;

                let filter = if needs_and {
                    let mut and_builder = LifecycleRuleAndOperator::builder();
                    if let Some(ref prefix) = rule.prefix {
                        if !prefix.is_empty() {
                            and_builder = and_builder.prefix(prefix.clone());
                        }
                    }
                    if let Some(ref tags) = rule.tags {
                        for tag in tags {
                            let aws_tag = AwsTag::builder()
                                .key(tag.key.clone())
                                .value(tag.value.clone())
                                .build()
                                .expect("valid tag");
                            and_builder = and_builder.tags(aws_tag);
                        }
                    }
                    if let Some(size) = rule.object_size_greater_than {
                        and_builder = and_builder.object_size_greater_than(size);
                    }
                    if let Some(size) = rule.object_size_less_than {
                        and_builder = and_builder.object_size_less_than(size);
                    }
                    Some(
                        LifecycleRuleFilter::builder()
                            .and(and_builder.build())
                            .build(),
                    )
                } else if has_prefix {
                    Some(
                        LifecycleRuleFilter::builder()
                            .prefix(rule.prefix.clone().unwrap_or_default())
                            .build(),
                    )
                } else {
                    Some(LifecycleRuleFilter::builder().prefix(String::new()).build())
                };

                let expiration = rule
                    .expiration_days
                    .map(|days| LifecycleExpiration::builder().days(days).build());

                let noncurrent = rule.noncurrent_version_expiration_days.map(|days| {
                    NoncurrentVersionExpiration::builder()
                        .noncurrent_days(days)
                        .build()
                });

                let mut builder = AwsLifecycleRule::builder().id(rule.id).status(status);
                if let Some(f) = filter {
                    builder = builder.filter(f);
                }
                if let Some(exp) = expiration {
                    builder = builder.expiration(exp);
                }
                if let Some(nve) = noncurrent {
                    builder = builder.noncurrent_version_expiration(nve);
                }
                builder.build().expect("valid lifecycle rule")
            })
            .collect();

        let config = BucketLifecycleConfiguration::builder()
            .set_rules(Some(aws_rules))
            .build()
            .expect("valid lifecycle configuration");

        client
            .put_bucket_lifecycle_configuration()
            .bucket(bucket_name)
            .lifecycle_configuration(config)
            .send()
            .await
            .map_err(|e| ObsError::S3Error(format!("Failed to set lifecycle rules: {:?}", e)))?;

        Ok(())
    }

    /// Delete all lifecycle rules from a bucket.
    pub async fn delete_lifecycle_rules(&self, bucket_name: &str) -> Result<()> {
        let bucket = self.get_bucket(bucket_name)?;

        match bucket.delete_bucket_lifecycle().await {
            Ok(_) => Ok(()),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("NoSuchLifecycleConfiguration")
                    || err_str.contains("404")
                    || err_str.contains("NoSuchConfiguration")
                {
                    Ok(())
                } else {
                    Err(ObsError::S3Error(err_str))
                }
            }
        }
    }
}

// ─── CORS Management ─────────────────────────────────────────────────────────

impl ObsClient {
    /// Get CORS rules for a bucket. Returns an empty vec if none are configured.
    pub async fn get_bucket_cors(&self, bucket_name: &str) -> Result<Vec<CorsRule>> {
        let client = self.get_aws_client().await?;

        match client.get_bucket_cors().bucket(bucket_name).send().await {
            Ok(output) => {
                let rules = output
                    .cors_rules()
                    .iter()
                    .enumerate()
                    .map(|(i, rule)| CorsRule {
                        id: rule
                            .id()
                            .unwrap_or(&format!("rule-{}", i + 1))
                            .to_string(),
                        allowed_origins: rule.allowed_origins().to_vec(),
                        allowed_methods: rule.allowed_methods().to_vec(),
                        allowed_headers: if rule.allowed_headers().is_empty() {
                            None
                        } else {
                            Some(rule.allowed_headers().to_vec())
                        },
                        expose_headers: if rule.expose_headers().is_empty() {
                            None
                        } else {
                            Some(rule.expose_headers().to_vec())
                        },
                        max_age_seconds: rule.max_age_seconds(),
                    })
                    .collect();
                Ok(rules)
            }
            Err(e) => {
                let err_str = format!("{:?}", e);
                if err_str.contains("NoSuchCORSConfiguration")
                    || err_str.contains("404")
                    || err_str.contains("NoSuchConfiguration")
                    || err_str.contains("CORS configuration does not exist")
                {
                    Ok(vec![])
                } else {
                    Err(ObsError::Other(format!(
                        "Failed to get CORS configuration: {:?}",
                        e
                    )))
                }
            }
        }
    }

    /// Set CORS rules for a bucket (replaces all existing rules).
    ///
    /// Automatically computes the `Content-MD5` header required by Huawei OBS.
    pub async fn put_bucket_cors(
        &self,
        bucket_name: &str,
        rules: Vec<CorsRule>,
    ) -> Result<()> {
        use aws_sdk_s3::types::{CorsConfiguration, CorsRule as AwsCorsRule};
        use base64::Engine;
        use md5::{Digest, Md5};

        let client = self.get_aws_client().await?;

        let aws_rules: Vec<AwsCorsRule> = rules
            .into_iter()
            .map(|rule| {
                let mut builder = AwsCorsRule::builder()
                    .set_id(Some(rule.id))
                    .set_allowed_origins(Some(rule.allowed_origins))
                    .set_allowed_methods(Some(rule.allowed_methods))
                    .set_allowed_headers(rule.allowed_headers)
                    .set_expose_headers(rule.expose_headers);

                if let Some(max_age) = rule.max_age_seconds {
                    builder = builder.max_age_seconds(max_age);
                }

                builder.build().expect("Failed to build CORS rule")
            })
            .collect();

        let cors_config = CorsConfiguration::builder()
            .set_cors_rules(Some(aws_rules))
            .build()
            .expect("Failed to build CORS configuration");

        client
            .put_bucket_cors()
            .bucket(bucket_name)
            .cors_configuration(cors_config)
            .customize()
            .mutate_request(|req| {
                if let Some(body_bytes) = req.body().bytes() {
                    let digest = Md5::digest(body_bytes);
                    let md5_b64 = base64::engine::general_purpose::STANDARD.encode(digest);
                    req.headers_mut().insert("content-md5", md5_b64);
                }
            })
            .send()
            .await
            .map_err(|e| {
                ObsError::Other(format!("Failed to set CORS configuration: {:?}", e))
            })?;

        Ok(())
    }

    /// Delete all CORS rules from a bucket.
    pub async fn delete_bucket_cors(&self, bucket_name: &str) -> Result<()> {
        let client = self.get_aws_client().await?;

        match client
            .delete_bucket_cors()
            .bucket(bucket_name)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                let err_str = format!("{:?}", e);
                if err_str.contains("NoSuchCORSConfiguration")
                    || err_str.contains("404")
                    || err_str.contains("NoSuchConfiguration")
                {
                    Ok(())
                } else {
                    Err(ObsError::Other(format!(
                        "Failed to delete CORS configuration: {:?}",
                        e
                    )))
                }
            }
        }
    }
}

// ─── Policy Management ───────────────────────────────────────────────────────

impl ObsClient {
    /// Get the bucket policy (JSON). Returns an empty string if no policy is set.
    pub async fn get_bucket_policy(&self, bucket_name: &str) -> Result<BucketPolicy> {
        let client = self.get_aws_client().await?;

        match client
            .get_bucket_policy()
            .bucket(bucket_name)
            .send()
            .await
        {
            Ok(output) => Ok(BucketPolicy {
                policy_json: output.policy().unwrap_or("").to_string(),
            }),
            Err(e) => {
                let err_str = format!("{:?}", e);
                if err_str.contains("NoSuchBucketPolicy")
                    || err_str.contains("404")
                    || err_str.contains("NoSuchPolicy")
                    || err_str.contains("The bucket policy does not exist")
                    || err_str.contains("policy")
                {
                    Ok(BucketPolicy {
                        policy_json: String::new(),
                    })
                } else {
                    Err(ObsError::Other(format!(
                        "Failed to get bucket policy: {:?}",
                        e
                    )))
                }
            }
        }
    }

    /// Set the bucket policy (JSON).
    pub async fn put_bucket_policy(
        &self,
        bucket_name: &str,
        policy: BucketPolicy,
    ) -> Result<()> {
        let client = self.get_aws_client().await?;

        client
            .put_bucket_policy()
            .bucket(bucket_name)
            .policy(policy.policy_json)
            .send()
            .await
            .map_err(|e| ObsError::Other(format!("Failed to set bucket policy: {:?}", e)))?;

        Ok(())
    }

    /// Delete the bucket policy.
    pub async fn delete_bucket_policy(&self, bucket_name: &str) -> Result<()> {
        let client = self.get_aws_client().await?;

        match client
            .delete_bucket_policy()
            .bucket(bucket_name)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                let err_str = format!("{:?}", e);
                if err_str.contains("NoSuchBucketPolicy")
                    || err_str.contains("404")
                    || err_str.contains("NoSuchPolicy")
                    || err_str.contains("The bucket policy does not exist")
                {
                    Ok(())
                } else {
                    Err(ObsError::Other(format!(
                        "Failed to delete bucket policy: {:?}",
                        e
                    )))
                }
            }
        }
    }
}

// ─── Object Lock / WORM ─────────────────────────────────────────────────────

impl ObsClient {
    /// Get the Object Lock configuration for a bucket.
    pub async fn get_object_lock_configuration(
        &self,
        bucket_name: &str,
    ) -> Result<ObjectLockConfiguration> {
        let bucket = self.get_bucket(bucket_name)?;
        let url = format!("{}/?object-lock", bucket.url());

        let http = reqwest::Client::new();
        let response = http
            .get(&url)
            .header("Date", chrono::Utc::now().to_rfc2822())
            .send()
            .await
            .map_err(|e| ObsError::Other(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Ok(ObjectLockConfiguration {
                    enabled: false,
                    default_retention: None,
                });
            }
            return Err(ObsError::Other(format!(
                "Failed to get object lock config: {}",
                response.status()
            )));
        }

        let xml_text = response
            .text()
            .await
            .map_err(|e| ObsError::Other(format!("Failed to read response: {}", e)))?;

        Ok(ObjectLockConfiguration {
            enabled: xml_text.contains("<ObjectLockEnabled>Enabled</ObjectLockEnabled>"),
            default_retention: None,
        })
    }

    /// Set Object Lock configuration for a bucket.
    pub async fn put_object_lock_configuration(
        &self,
        bucket_name: &str,
        config: ObjectLockConfiguration,
    ) -> Result<()> {
        let bucket = self.get_bucket(bucket_name)?;
        let url = format!("{}/?object-lock", bucket.url());

        let mut xml =
            String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<ObjectLockConfiguration xmlns=\"http://obs.myhuaweicloud.com/doc/2015-06-30/\">\n");

        if config.enabled {
            xml.push_str("    <ObjectLockEnabled>Enabled</ObjectLockEnabled>\n");

            if let Some(retention) = &config.default_retention {
                xml.push_str("    <Rule>\n");
                xml.push_str("        <DefaultRetention>\n");
                xml.push_str("            <Mode>COMPLIANCE</Mode>\n");

                if let Some(days) = retention.days {
                    xml.push_str(&format!("            <Days>{}</Days>\n", days));
                } else if let Some(years) = retention.years {
                    xml.push_str(&format!("            <Years>{}</Years>\n", years));
                }

                xml.push_str("        </DefaultRetention>\n");
                xml.push_str("    </Rule>\n");
            }
        }

        xml.push_str("</ObjectLockConfiguration>");

        let http = reqwest::Client::new();
        let response = http
            .put(&url)
            .header("Content-Type", "application/xml")
            .header("Date", chrono::Utc::now().to_rfc2822())
            .body(xml)
            .send()
            .await
            .map_err(|e| ObsError::Other(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ObsError::Other(format!(
                "Failed to set object lock config: {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Delete Object Lock configuration from a bucket.
    pub async fn delete_object_lock_configuration(&self, bucket_name: &str) -> Result<()> {
        self.put_object_lock_configuration(
            bucket_name,
            ObjectLockConfiguration {
                enabled: false,
                default_retention: None,
            },
        )
        .await
    }

    /// Get WORM retention for a specific object.
    pub async fn get_object_retention(
        &self,
        bucket_name: &str,
        key: &str,
        version_id: Option<&str>,
    ) -> Result<WormRetention> {
        let bucket = self.get_bucket(bucket_name)?;
        let mut url = format!("{}/{}?retention", bucket.url(), key);
        if let Some(vid) = version_id {
            url.push_str(&format!("&versionId={}", vid));
        }

        let http = reqwest::Client::new();
        let response = http
            .get(&url)
            .header("Date", chrono::Utc::now().to_rfc2822())
            .send()
            .await
            .map_err(|e| ObsError::Other(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ObsError::Other(format!(
                "Failed to get object retention: {}",
                response.status()
            )));
        }

        // Simplified parsing — a production version would use quick-xml
        Ok(WormRetention {
            mode: WormRetentionMode::Compliance,
            retain_until_date: 0,
        })
    }

    /// Set WORM retention for a specific object.
    pub async fn put_object_retention(
        &self,
        bucket_name: &str,
        key: &str,
        retention: WormRetention,
        version_id: Option<&str>,
    ) -> Result<()> {
        let bucket = self.get_bucket(bucket_name)?;
        let mut url = format!("{}/{}?retention", bucket.url(), key);
        if let Some(vid) = version_id {
            url.push_str(&format!("&versionId={}", vid));
        }

        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Retention>
    <Mode>COMPLIANCE</Mode>
    <RetainUntilDate>{}</RetainUntilDate>
</Retention>"#,
            retention.retain_until_date
        );

        let http = reqwest::Client::new();
        let response = http
            .put(&url)
            .header("Content-Type", "application/xml")
            .header("Date", chrono::Utc::now().to_rfc2822())
            .body(xml)
            .send()
            .await
            .map_err(|e| ObsError::Other(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ObsError::Other(format!(
                "Failed to set object retention: {}",
                error_text
            )));
        }

        Ok(())
    }
}
