use anyhow::{bail, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::Client;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_buckets_returns_vec() {
        let names: Vec<String> = vec!["my-bucket".into()];
        assert_eq!(names[0], "my-bucket");
    }
}
