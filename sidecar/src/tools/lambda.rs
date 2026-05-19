use anyhow::Result;
use serde_json::{json, Value};

use crate::auth::state::SharedState;
use crate::mcp::dispatcher::Dispatcher;
use crate::services::cloudwatch as cw_svc;
use crate::services::lambda as svc;

pub fn register(dispatcher: &mut Dispatcher, state: SharedState) {
    let s1 = state.clone();
    dispatcher.register("lambda_list", Box::new(move |_params| {
        let state = s1.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                lambda_list_handler(state).await
            })
        })
    }));

    let s2 = state.clone();
    dispatcher.register("lambda_invoke", Box::new(move |params| {
        let state = s2.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                lambda_invoke_handler(state, params).await
            })
        })
    }));

    let s3 = state.clone();
    dispatcher.register("lambda_get_logs", Box::new(move |params| {
        let state = s3.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                lambda_get_logs_handler(state, params).await
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

async fn lambda_list_handler(state: SharedState) -> Result<Value> {
    let profile = active_profile(&state)?;
    let client = svc::build_client(&profile).await?;
    let functions = svc::list_functions(&client).await?;
    Ok(json!({ "functions": functions }))
}

async fn lambda_invoke_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let name = params["name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: name"))?;
    let payload = params.get("payload").cloned().unwrap_or(json!({}));
    let client = svc::build_client(&profile).await?;
    let result = svc::invoke_function(&client, name, payload).await?;
    Ok(result)
}

async fn lambda_get_logs_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let name = params["name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: name"))?;
    let group = format!("/aws/lambda/{}", name);
    let since_ms = params["since"].as_i64().map(|secs_ago| {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        now_ms - secs_ago * 1000
    });
    let cw_client = cw_svc::build_client(&profile).await?;
    let mut events = cw_svc::tail(&cw_client, &group, None, since_ms).await?;
    if let Some(n) = params["tail"].as_i64() {
        let n = n.max(0) as usize;
        if events.len() > n {
            events = events.into_iter().rev().take(n).rev().collect();
        }
    }
    Ok(json!({ "group": group, "events": events }))
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
    fn lambda_log_group_name() {
        let name = "my-function";
        let group = format!("/aws/lambda/{}", name);
        assert_eq!(group, "/aws/lambda/my-function");
    }

    #[test]
    fn missing_name_param() {
        let params = json!({});
        let result: Result<&str> = params["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing required param: name"));
        assert!(result.unwrap_err().to_string().contains("name"));
    }
}
