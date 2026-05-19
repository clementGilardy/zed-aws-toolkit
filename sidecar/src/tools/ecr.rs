use anyhow::Result;
use serde_json::{json, Value};

use crate::auth::state::SharedState;
use crate::mcp::dispatcher::Dispatcher;
use crate::services::ecr as svc;

pub fn register(dispatcher: &mut Dispatcher, state: SharedState) {
    let s1 = state.clone();
    dispatcher.register("ecr_list_repos", Box::new(move |_params| {
        let state = s1.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                ecr_list_repos_handler(state).await
            })
        })
    }));

    let s2 = state;
    dispatcher.register("ecr_list_images", Box::new(move |params| {
        let state = s2.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                ecr_list_images_handler(state, params).await
            })
        })
    }));
}

fn active_profile(state: &SharedState) -> Result<crate::auth::config::SsoProfile> {
    state.lock().unwrap().active_profile.clone()
        .ok_or_else(|| anyhow::anyhow!("No active AWS profile. Run list_accounts then switch_account first."))
}

async fn ecr_list_repos_handler(state: SharedState) -> Result<Value> {
    let profile = active_profile(&state)?;
    let client = svc::build_client(&profile).await?;
    let repos = svc::list_repos(&client).await?;
    Ok(json!({ "repos": repos }))
}

async fn ecr_list_images_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let repo = params["repo"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: repo"))?;
    let max = params["max"].as_i64().map(|v| v as i32);
    let client = svc::build_client(&profile).await?;
    let images = svc::list_images(&client, repo, max).await?;
    Ok(json!({ "images": images }))
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
    fn missing_repo_param_errors() {
        let params = json!({});
        let result: Result<&str> = params["repo"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing required param: repo"));
        assert!(result.unwrap_err().to_string().contains("repo"));
    }

    #[test]
    fn max_param_absent_is_none() {
        let params = json!({ "repo": "my-app" });
        let max = params["max"].as_i64().map(|v| v as i32);
        assert!(max.is_none());
    }

    #[test]
    fn max_param_present_parsed_correctly() {
        let params = json!({ "repo": "my-app", "max": 5 });
        let max = params["max"].as_i64().map(|v| v as i32);
        assert_eq!(max, Some(5));
    }
}
