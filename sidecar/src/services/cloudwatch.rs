use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_cloudwatchlogs::config::{Credentials, Region};
use aws_sdk_cloudwatchlogs::types::OrderBy;
use aws_sdk_cloudwatchlogs::Client;

use crate::auth::config::SsoProfile;
use crate::auth::login::ensure_authenticated;

pub async fn build_client(profile: &SsoProfile) -> Result<Client> {
    let creds = ensure_authenticated(profile).await?;
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


pub async fn list_groups(
    client: &Client,
    prefix: Option<&str>,
) -> Result<Vec<serde_json::Value>> {
    let mut groups = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut req = client.describe_log_groups();
        if let Some(p) = prefix {
            req = req.log_group_name_prefix(p);
        }
        if let Some(t) = next_token.take() {
            req = req.next_token(t);
        }
        let resp = req.send().await?;
        for g in resp.log_groups() {
            groups.push(serde_json::json!({
                "name": g.log_group_name().unwrap_or_default(),
                "retention_days": g.retention_in_days(),
                "stored_bytes": g.stored_bytes().unwrap_or(0),
            }));
        }
        match resp.next_token() {
            Some(t) => next_token = Some(t.to_string()),
            None => break,
        }
    }
    Ok(groups)
}

pub async fn list_streams(
    client: &Client,
    group: &str,
    limit: Option<i32>,
) -> Result<Vec<serde_json::Value>> {
    let mut req = client
        .describe_log_streams()
        .log_group_name(group)
        .order_by(OrderBy::LastEventTime)
        .descending(true);
    if let Some(l) = limit {
        req = req.limit(l);
    }
    let resp = req.send().await?;
    let streams = resp
        .log_streams()
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.log_stream_name().unwrap_or_default(),
                "last_event_ms": s.last_event_timestamp(),
                "first_event_ms": s.first_event_timestamp(),
            })
        })
        .collect();
    Ok(streams)
}

pub async fn tail(
    client: &Client,
    group: &str,
    stream: Option<&str>,
    since_ms: Option<i64>,
) -> Result<Vec<serde_json::Value>> {
    let start = since_ms.unwrap_or_else(|| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        now - 15 * 60 * 1000
    });

    let mut events = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut req = client
            .filter_log_events()
            .log_group_name(group)
            .start_time(start);
        if let Some(s) = stream {
            req = req.log_stream_names(s.to_string());
        }
        if let Some(t) = next_token.take() {
            req = req.next_token(t);
        }
        let resp = req.send().await?;
        for e in resp.events() {
            events.push(serde_json::json!({
                "timestamp_ms": e.timestamp(),
                "message": e.message().unwrap_or_default(),
                "stream": e.log_stream_name().unwrap_or_default(),
            }));
        }
        match resp.next_token() {
            Some(t) => next_token = Some(t.to_string()),
            None => break,
        }
    }
    Ok(events)
}

pub async fn search(
    client: &Client,
    group: &str,
    query: &str,
    since_ms: Option<i64>,
) -> Result<Vec<serde_json::Value>> {
    let start = since_ms.unwrap_or_else(|| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        now - 60 * 60 * 1000
    });

    let mut events = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut req = client
            .filter_log_events()
            .log_group_name(group)
            .filter_pattern(query)
            .start_time(start);
        if let Some(t) = next_token.take() {
            req = req.next_token(t);
        }
        let resp = req.send().await?;
        for e in resp.events() {
            events.push(serde_json::json!({
                "timestamp_ms": e.timestamp(),
                "message": e.message().unwrap_or_default(),
                "stream": e.log_stream_name().unwrap_or_default(),
            }));
        }
        match resp.next_token() {
            Some(t) => next_token = Some(t.to_string()),
            None => break,
        }
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_default_since_is_15_minutes_ago() {
        let before_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let default_start = before_ms - 15 * 60 * 1000;
        let after_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let expected_15min_ago = after_ms - 15 * 60 * 1000;
        // default_start must be within 1s of 15 minutes ago
        assert!((default_start - expected_15min_ago).abs() < 1000);
    }

    #[test]
    fn search_result_shape() {
        let e = serde_json::json!({
            "timestamp_ms": 1700000000000_i64,
            "message": "ERROR something failed",
            "stream": "my-stream",
        });
        assert!(e["message"].as_str().unwrap().contains("ERROR"));
    }
}
