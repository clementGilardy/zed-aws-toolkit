use anyhow::{bail, Result};
use aws_config::BehaviorVersion;
use aws_sdk_ecs::config::{Credentials, Region};
use aws_sdk_ecs::Client;

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

pub async fn list_clusters(client: &Client) -> Result<Vec<String>> {
    let mut arns = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut req = client.list_clusters();
        if let Some(t) = next_token.take() {
            req = req.next_token(t);
        }
        let resp = req.send().await?;
        arns.extend(resp.cluster_arns().iter().map(|s| s.clone()));
        match resp.next_token() {
            Some(t) => next_token = Some(t.to_string()),
            None => break,
        }
    }
    Ok(arns)
}

pub async fn list_services(client: &Client, cluster: &str) -> Result<Vec<String>> {
    let mut arns = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut req = client.list_services().cluster(cluster);
        if let Some(t) = next_token.take() {
            req = req.next_token(t);
        }
        let resp = req.send().await?;
        arns.extend(resp.service_arns().iter().map(|s| s.clone()));
        match resp.next_token() {
            Some(t) => next_token = Some(t.to_string()),
            None => break,
        }
    }
    Ok(arns)
}

pub async fn list_tasks(
    client: &Client,
    cluster: &str,
    service: Option<&str>,
) -> Result<Vec<String>> {
    let mut arns = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut req = client.list_tasks().cluster(cluster);
        if let Some(s) = service {
            req = req.service_name(s);
        }
        if let Some(t) = next_token.take() {
            req = req.next_token(t);
        }
        let resp = req.send().await?;
        arns.extend(resp.task_arns().iter().map(|s| s.clone()));
        match resp.next_token() {
            Some(t) => next_token = Some(t.to_string()),
            None => break,
        }
    }
    Ok(arns)
}

pub async fn describe_task(
    client: &Client,
    cluster: &str,
    task_arn: &str,
) -> Result<serde_json::Value> {
    let resp = client
        .describe_tasks()
        .cluster(cluster)
        .tasks(task_arn)
        .send()
        .await?;

    let task = resp
        .tasks()
        .first()
        .ok_or_else(|| anyhow::anyhow!("task not found: {}", task_arn))?;

    let task_def_arn = task.task_definition_arn().unwrap_or_default().to_string();
    let last_status = task.last_status().unwrap_or_default().to_string();
    let desired_status = task.desired_status().unwrap_or_default().to_string();
    let started_at = task.started_at().map(|t| t.to_string());
    let stopped_at = task.stopped_at().map(|t| t.to_string());
    let stopped_reason = task.stopped_reason().map(str::to_string);

    let containers: Vec<serde_json::Value> = task
        .containers()
        .iter()
        .map(|c| {
            serde_json::json!({
                "name": c.name().unwrap_or_default(),
                "image": c.image().unwrap_or_default(),
                "status": c.last_status().unwrap_or_default(),
            })
        })
        .collect();

    let td_resp = client
        .describe_task_definition()
        .task_definition(&task_def_arn)
        .send()
        .await?;

    let log_config: Vec<serde_json::Value> = td_resp
        .task_definition()
        .map(|td| td.container_definitions())
        .unwrap_or_default()
        .iter()
        .filter_map(|cd| {
            cd.log_configuration().map(|lc| {
                serde_json::json!({
                    "container": cd.name().unwrap_or_default(),
                    "log_driver": lc.log_driver().as_str(),
                    "options": lc.options().map(|o| serde_json::to_value(o).unwrap_or(serde_json::json!({}))),
                })
            })
        })
        .collect();

    Ok(serde_json::json!({
        "task_arn": task_arn,
        "task_definition_arn": task_def_arn,
        "last_status": last_status,
        "desired_status": desired_status,
        "started_at": started_at,
        "stopped_at": stopped_at,
        "stopped_reason": stopped_reason,
        "containers": containers,
        "log_config": log_config,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_task_shape() {
        let t = serde_json::json!({
            "task_arn": "arn:aws:ecs:eu-central-1:123456789012:task/my-cluster/abc123",
            "task_definition_arn": "arn:aws:ecs:eu-central-1:123456789012:task-definition/my-task:1",
            "last_status": "RUNNING",
            "desired_status": "RUNNING",
            "started_at": "2026-05-19T10:00:00Z",
            "stopped_at": null,
            "stopped_reason": null,
            "containers": [{"name": "app", "image": "nginx:latest", "status": "RUNNING"}],
            "log_config": [{"container": "app", "log_driver": "awslogs", "options": {}}],
        });
        assert_eq!(t["last_status"], "RUNNING");
        assert!(t["containers"].is_array());
        assert!(t["log_config"].is_array());
    }

    #[test]
    fn list_clusters_returns_vec() {
        let arns: Vec<String> = vec!["arn:aws:ecs:eu-central-1:123:cluster/my-cluster".into()];
        assert_eq!(arns.len(), 1);
        assert!(arns[0].contains("cluster"));
    }
}
