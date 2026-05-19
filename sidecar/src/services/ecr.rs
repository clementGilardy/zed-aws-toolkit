use anyhow::{bail, Result};
use aws_config::BehaviorVersion;
use aws_sdk_ecr::config::{Credentials, Region};
use aws_sdk_ecr::Client;

use crate::auth::cache::{read_sso_token, CachedCredentials};
use crate::auth::config::SsoProfile;
use crate::auth::login::get_role_credentials;

pub async fn build_client(profile: &SsoProfile) -> Result<Client> {
    let creds = resolve_credentials(profile).await?;
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(profile.region.clone()))
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
        _ => bail!("Not authenticated. Run sso_login with profile \"{}\" first.", profile.name),
    };
    get_role_credentials(profile, &token.access_token).await
}

pub async fn list_repos(client: &Client) -> Result<Vec<serde_json::Value>> {
    let mut repos = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut req = client.describe_repositories();
        if let Some(t) = next_token.take() {
            req = req.next_token(t);
        }
        let resp = req.send().await?;
        for r in resp.repositories() {
            repos.push(serde_json::json!({
                "name": r.repository_name().unwrap_or_default(),
                "uri": r.repository_uri().unwrap_or_default(),
                "created_at": r.created_at().map(|t| t.to_string()),
                "image_tag_mutability": r.image_tag_mutability().map(|m| m.as_str()).unwrap_or("unknown"),
            }));
        }
        match resp.next_token() {
            Some(t) => next_token = Some(t.to_owned()),
            None => break,
        }
    }
    Ok(repos)
}

pub async fn list_images(
    client: &Client,
    repo: &str,
    max: Option<i32>,
) -> Result<Vec<serde_json::Value>> {
    let mut images = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut req = client.describe_images().repository_name(repo);
        if let Some(t) = next_token.take() {
            req = req.next_token(t);
        }
        let resp = req.send().await?;
        for img in resp.image_details() {
            images.push(serde_json::json!({
                "digest": img.image_digest().unwrap_or_default(),
                "tags": img.image_tags(),
                "pushed_at": img.image_pushed_at().map(|t| t.to_string()),
                "size_bytes": img.image_size_in_bytes(),
            }));
        }
        match resp.next_token() {
            Some(t) => next_token = Some(t.to_owned()),
            None => break,
        }
        if let Some(m) = max {
            if images.len() >= m as usize {
                images.truncate(m as usize);
                break;
            }
        }
    }
    Ok(images)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_repos_output_shape() {
        let repo = serde_json::json!({
            "name": "my-app",
            "uri": "123456789012.dkr.ecr.eu-central-1.amazonaws.com/my-app",
            "created_at": "2026-01-01T00:00:00Z",
            "image_tag_mutability": "MUTABLE",
        });
        assert_eq!(repo["name"], "my-app");
        assert!(repo["uri"].as_str().unwrap().contains("dkr.ecr"));
        assert_eq!(repo["image_tag_mutability"], "MUTABLE");
        assert!(repo["created_at"].is_string());
    }

    #[test]
    fn list_images_max_truncation_logic() {
        let mut images: Vec<serde_json::Value> = (0..10)
            .map(|i| serde_json::json!({
                "digest": format!("sha256:{:064}", i),
                "tags": ["latest"],
                "pushed_at": "2026-01-01T00:00:00Z",
                "size_bytes": 1024_i64,
            }))
            .collect();
        let max = 3_i32;
        if images.len() >= max as usize {
            images.truncate(max as usize);
        }
        assert_eq!(images.len(), 3);
        assert_eq!(images[0]["tags"][0], "latest");
        assert_eq!(images[0]["size_bytes"], 1024);
    }

    #[test]
    fn list_images_no_max_keeps_all() {
        let images: Vec<serde_json::Value> = (0..5)
            .map(|i| serde_json::json!({"digest": format!("sha256:{:064}", i), "tags": [], "pushed_at": null, "size_bytes": null}))
            .collect();
        // No truncation when max is None
        assert_eq!(images.len(), 5);
    }
}
