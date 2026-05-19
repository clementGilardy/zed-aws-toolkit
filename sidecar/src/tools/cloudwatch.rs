use anyhow::Result;
use serde_json::{json, Value};

use crate::auth::state::SharedState;
use crate::mcp::dispatcher::Dispatcher;
use crate::services::cloudwatch as svc;

pub fn register(dispatcher: &mut Dispatcher, state: SharedState) {
    let s1 = state.clone();
    dispatcher.register("logs_list_groups", Box::new(move |params| {
        let state = s1.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                logs_list_groups_handler(state, params).await
            })
        })
    }));

    let s2 = state.clone();
    dispatcher.register("logs_list_streams", Box::new(move |params| {
        let state = s2.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                logs_list_streams_handler(state, params).await
            })
        })
    }));

    let s3 = state.clone();
    dispatcher.register("logs_tail", Box::new(move |params| {
        let state = s3.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                logs_tail_handler(state, params).await
            })
        })
    }));

    let s4 = state.clone();
    dispatcher.register("logs_search", Box::new(move |params| {
        let state = s4.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                logs_search_handler(state, params).await
            })
        })
    }));
}

fn active_profile(state: &SharedState) -> Result<crate::auth::config::SsoProfile> {
    state
        .lock()
        .unwrap()
        .active_profile
        .clone()
        .ok_or_else(|| anyhow::anyhow!(
            "No active AWS profile. Run list_accounts then switch_account first."
        ))
}

async fn logs_list_groups_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let prefix = params["prefix"].as_str();
    let client = svc::build_client(&profile).await?;
    let groups = svc::list_groups(&client, prefix).await?;
    Ok(json!({ "groups": groups }))
}

async fn logs_list_streams_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let group = params["group"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: group"))?;
    let limit = params["limit"].as_i64().map(|v| v as i32);
    let client = svc::build_client(&profile).await?;
    let streams = svc::list_streams(&client, group, limit).await?;
    Ok(json!({ "streams": streams }))
}

async fn logs_tail_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let group = params["group"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: group"))?;
    let stream = params["stream"].as_str();
    let since_ms = params["since"].as_i64().map(|secs_ago| {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        now_ms - secs_ago * 1000
    });
    let client = svc::build_client(&profile).await?;
    let events = svc::tail(&client, group, stream, since_ms).await?;
    Ok(json!({ "events": events }))
}

async fn logs_search_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let group = params["group"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: group"))?;
    let query = params["query"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: query"))?;
    let since_ms = params["since"].as_i64().map(|secs_ago| {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        now_ms - secs_ago * 1000
    });
    let client = svc::build_client(&profile).await?;
    let events = svc::search(&client, group, query, since_ms).await?;
    Ok(json!({ "events": events }))
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
    fn missing_group_param() {
        let params = json!({});
        let result: Result<&str> = params["group"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing required param: group"));
        assert!(result.unwrap_err().to_string().contains("group"));
    }

    #[test]
    fn missing_query_param() {
        let params = json!({});
        let result: Result<&str> = params["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing required param: query"));
        assert!(result.unwrap_err().to_string().contains("query"));
    }
}
