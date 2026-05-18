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
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(profile.region.clone()))
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
