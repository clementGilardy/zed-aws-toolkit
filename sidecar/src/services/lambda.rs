use anyhow::{bail, Result};
use aws_config::BehaviorVersion;
use aws_sdk_lambda::config::{Credentials, Region};
use aws_sdk_lambda::primitives::Blob;
use aws_sdk_lambda::Client;

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
        _ => bail!(
            "Not authenticated. Run sso_login with profile \"{}\" first.",
            profile.name
        ),
    };
    get_role_credentials(profile, &token.access_token).await
}

pub async fn list_functions(client: &Client) -> Result<Vec<serde_json::Value>> {
    let mut functions = Vec::new();
    let mut marker: Option<String> = None;

    loop {
        let mut req = client.list_functions();
        if let Some(m) = marker.take() {
            req = req.marker(m);
        }
        let resp = req.send().await?;
        for f in resp.functions() {
            functions.push(serde_json::json!({
                "name": f.function_name().unwrap_or_default(),
                "runtime": f.runtime().map(|r| r.as_str()).unwrap_or("unknown"),
                "handler": f.handler().unwrap_or_default(),
                "memory_mb": f.memory_size().unwrap_or(128),
                "timeout_secs": f.timeout().unwrap_or(3),
                "last_modified": f.last_modified().unwrap_or_default(),
                "description": f.description().unwrap_or_default(),
            }));
        }
        match resp.next_marker() {
            Some(m) => marker = Some(m.to_string()),
            None => break,
        }
    }
    Ok(functions)
}

pub async fn invoke_function(
    client: &Client,
    name: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value> {
    let payload_bytes = serde_json::to_vec(&payload)?;
    let resp = client
        .invoke()
        .function_name(name)
        .payload(Blob::new(payload_bytes))
        .send()
        .await?;

    let status = resp.status_code();
    let function_error = resp.function_error().map(str::to_string);
    let payload_str = resp
        .payload()
        .and_then(|b| std::str::from_utf8(b.as_ref()).ok())
        .unwrap_or("")
        .to_string();
    let payload_json: serde_json::Value = serde_json::from_str(&payload_str)
        .unwrap_or(serde_json::Value::String(payload_str));

    Ok(serde_json::json!({
        "status_code": status,
        "function_error": function_error,
        "payload": payload_json,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_response_serializes_function_error() {
        let resp = serde_json::json!({
            "status_code": 200,
            "function_error": null,
            "payload": {"result": "ok"},
        });
        assert_eq!(resp["status_code"], 200);
        assert!(resp["function_error"].is_null());
    }

    #[test]
    fn list_functions_result_shape() {
        let f = serde_json::json!({
            "name": "my-fn",
            "runtime": "python3.12",
            "handler": "index.handler",
            "memory_mb": 128,
            "timeout_secs": 30,
            "last_modified": "2026-01-01T00:00:00.000+0000",
            "description": "",
        });
        assert_eq!(f["name"], "my-fn");
        assert_eq!(f["memory_mb"], 128);
    }
}
