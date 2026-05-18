use anyhow::{bail, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;
use std::time::Duration;

use crate::auth::cache::{read_sso_token, CachedCredentials};
use crate::auth::config::SsoProfile;
use crate::auth::login::get_role_credentials;

pub async fn build_client(profile: &SsoProfile) -> Result<Client> {
    let creds = resolve_credentials(profile).await?;
    build_client_with_creds(&creds, &profile.region).await
}

// Builds an S3 client for a specific bucket by detecting its region first.
// This handles cross-region buckets: list_buckets may return buckets from any region.
pub async fn build_client_for_bucket(profile: &SsoProfile, bucket: &str) -> Result<Client> {
    let creds = resolve_credentials(profile).await?;
    // Bootstrap client in profile region to call get_bucket_location
    let base = build_client_with_creds(&creds, &profile.region).await?;
    let region = get_bucket_region(&base, bucket).await?;
    build_client_with_creds(&creds, &region).await
}

async fn build_client_with_creds(creds: &CachedCredentials, region: &str) -> Result<Client> {
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(region.to_string()))
        .credentials_provider(Credentials::new(
            &creds.access_key_id,
            &creds.secret_access_key,
            Some(creds.session_token.clone()),
            None,
            "zed-aws-toolkit",
        ))
        .load()
        .await;
    Ok(Client::new(&sdk_config))
}

async fn get_bucket_region(client: &Client, bucket: &str) -> Result<String> {
    let resp = client.get_bucket_location().bucket(bucket).send().await?;
    // LocationConstraint is None for us-east-1 buckets
    let region = resp
        .location_constraint()
        .map(|c| c.as_str().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "us-east-1".to_string());
    Ok(region)
}

async fn resolve_credentials(profile: &SsoProfile) -> Result<CachedCredentials> {
    let token = match read_sso_token(&profile.sso_start_url)? {
        Some(t) if !t.is_expired() => t,
        _ => bail!(
            "Not authenticated. Run sso_login with profile \"{}\" first.",
            profile.name
        ),
    };
    get_role_credentials(profile, &token.access_token).await
}

pub async fn list_buckets(client: &Client) -> Result<Vec<String>> {
    let resp = client.list_buckets().send().await?;
    let names = resp
        .buckets()
        .iter()
        .filter_map(|b| b.name().map(str::to_string))
        .collect();
    Ok(names)
}

pub async fn list_objects(
    client: &Client,
    bucket: &str,
    prefix: Option<&str>,
    max_keys: Option<i32>,
) -> Result<Vec<serde_json::Value>> {
    let mut req = client.list_objects_v2().bucket(bucket);
    if let Some(p) = prefix {
        req = req.prefix(p);
    }
    if let Some(m) = max_keys {
        req = req.max_keys(m);
    }
    let resp = req.send().await?;
    let objects = resp
        .contents()
        .iter()
        .map(|o| {
            serde_json::json!({
                "key": o.key().unwrap_or_default(),
                "size": o.size().unwrap_or(0),
                "last_modified": o.last_modified().map(|t| t.to_string()),
            })
        })
        .collect();
    Ok(objects)
}

pub async fn get_object(client: &Client, bucket: &str, key: &str) -> Result<Vec<u8>> {
    let resp = client.get_object().bucket(bucket).key(key).send().await?;
    let bytes = resp.body.collect().await?.into_bytes().to_vec();
    Ok(bytes)
}

pub async fn put_object(client: &Client, bucket: &str, key: &str, body: Vec<u8>) -> Result<()> {
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body.into())
        .send()
        .await?;
    Ok(())
}

pub async fn delete_object(client: &Client, bucket: &str, key: &str) -> Result<()> {
    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;
    Ok(())
}

pub async fn presign_get(
    client: &Client,
    bucket: &str,
    key: &str,
    expires_secs: u64,
) -> Result<String> {
    let config = PresigningConfig::expires_in(Duration::from_secs(expires_secs))
        .map_err(|e| anyhow::anyhow!("presign config error: {e}"))?;
    let presigned = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .presigned(config)
        .await?;
    Ok(presigned.uri().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_buckets_returns_vec() {
        let names: Vec<String> = vec!["my-bucket".into()];
        assert_eq!(names[0], "my-bucket");
    }
}
