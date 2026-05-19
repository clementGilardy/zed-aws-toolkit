use anyhow::Result;
use serde_json::{json, Value};

use crate::auth::state::SharedState;
use crate::mcp::dispatcher::Dispatcher;
use crate::services::ecs as svc;

pub fn register(dispatcher: &mut Dispatcher, state: SharedState) {
    let s1 = state.clone();
    dispatcher.register("ecs_list_clusters", Box::new(move |_params| {
        let state = s1.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                ecs_list_clusters_handler(state).await
            })
        })
    }));

    let s2 = state.clone();
    dispatcher.register("ecs_list_services", Box::new(move |params| {
        let state = s2.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                ecs_list_services_handler(state, params).await
            })
        })
    }));

    let s3 = state.clone();
    dispatcher.register("ecs_list_tasks", Box::new(move |params| {
        let state = s3.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                ecs_list_tasks_handler(state, params).await
            })
        })
    }));

    let s4 = state.clone();
    dispatcher.register("ecs_describe_task", Box::new(move |params| {
        let state = s4.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                ecs_describe_task_handler(state, params).await
            })
        })
    }));
}

fn active_profile(state: &SharedState) -> Result<crate::auth::config::SsoProfile> {
    state.lock().unwrap().active_profile.clone()
        .ok_or_else(|| anyhow::anyhow!("No active AWS profile. Run list_accounts then switch_account first."))
}

async fn ecs_list_clusters_handler(state: SharedState) -> Result<Value> {
    let profile = active_profile(&state)?;
    let client = svc::build_client(&profile).await?;
    let clusters = svc::list_clusters(&client).await?;
    Ok(json!({ "clusters": clusters }))
}

async fn ecs_list_services_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let cluster = params["cluster"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: cluster"))?;
    let client = svc::build_client(&profile).await?;
    let services = svc::list_services(&client, cluster).await?;
    Ok(json!({ "services": services }))
}

async fn ecs_list_tasks_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let cluster = params["cluster"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: cluster"))?;
    let service = params["service"].as_str();
    let client = svc::build_client(&profile).await?;
    let tasks = svc::list_tasks(&client, cluster, service).await?;
    Ok(json!({ "tasks": tasks }))
}

async fn ecs_describe_task_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let cluster = params["cluster"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: cluster"))?;
    let task_arn = params["task_arn"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: task_arn"))?;
    let client = svc::build_client(&profile).await?;
    let detail = svc::describe_task(&client, cluster, task_arn).await?;
    Ok(detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::state::new_shared_state;

    #[test]
    fn active_profile_no_profile_errors() {
        let state = new_shared_state();
        let err = active_profile(&state).unwrap_err();
        assert!(err.to_string().contains("No active AWS profile"));
    }

    #[test]
    fn missing_cluster_param_errors() {
        let params = json!({});
        let result: Result<&str> = params["cluster"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing required param: cluster"));
        assert!(result.unwrap_err().to_string().contains("cluster"));
    }

    #[test]
    fn missing_task_arn_param_errors() {
        let params = json!({});
        let result: Result<&str> = params["task_arn"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing required param: task_arn"));
        assert!(result.unwrap_err().to_string().contains("task_arn"));
    }

    #[test]
    fn optional_service_param_is_none_when_absent() {
        let params = json!({ "cluster": "my-cluster" });
        let service = params["service"].as_str();
        assert!(service.is_none());
    }

    #[test]
    fn optional_service_param_is_some_when_present() {
        let params = json!({ "cluster": "my-cluster", "service": "my-service" });
        let service = params["service"].as_str();
        assert_eq!(service, Some("my-service"));
    }
}
