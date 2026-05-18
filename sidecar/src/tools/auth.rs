use anyhow::Result;
use serde_json::{json, Value};

use crate::auth::cache::clear_sso_token;
use crate::auth::config::{aws_config_path, parse_sso_profiles};
use crate::auth::login::sso_login;
use crate::auth::state::SharedState;
use crate::mcp::dispatcher::Dispatcher;

pub fn register(dispatcher: &mut Dispatcher, state: SharedState) {
    let s1 = state.clone();
    dispatcher.register(
        "list_accounts",
        Box::new(move |_params| list_accounts_handler(s1.clone())),
    );

    let s2 = state.clone();
    dispatcher.register(
        "switch_account",
        Box::new(move |params| switch_account_handler(s2.clone(), params)),
    );

    let s3 = state.clone();
    dispatcher.register(
        "sso_logout",
        Box::new(move |params| sso_logout_handler(s3.clone(), params)),
    );

    let s4 = state.clone();
    dispatcher.register(
        "sso_login",
        Box::new(move |params| {
            let profile_name = params["profile"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing required param: profile"))?
                .to_string();
            let state = s4.clone();
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async move {
                    sso_login_handler(state, &profile_name).await
                })
            })
        }),
    );
}

fn list_accounts_handler(state: SharedState) -> Result<Value> {
    let config_path = aws_config_path();
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", config_path.display()))?;
    let profiles = parse_sso_profiles(&content)?;
    let active = state
        .lock()
        .unwrap()
        .active_profile
        .as_ref()
        .map(|p| p.name.clone());
    let accounts: Vec<Value> = profiles
        .iter()
        .map(|p| {
            json!({
                "name": p.name,
                "account_id": p.sso_account_id,
                "role": p.sso_role_name,
                "region": p.region,
                "active": active.as_deref() == Some(p.name.as_str()),
            })
        })
        .collect();
    Ok(json!({ "accounts": accounts }))
}

fn switch_account_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile_name = params["profile"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: profile"))?
        .to_string();
    let config_path = aws_config_path();
    let content = std::fs::read_to_string(&config_path)?;
    let profiles = parse_sso_profiles(&content)?;
    let profile = profiles
        .into_iter()
        .find(|p| p.name == profile_name)
        .ok_or_else(|| anyhow::anyhow!("profile not found: {profile_name}"))?;
    state.lock().unwrap().active_profile = Some(profile);
    Ok(json!({ "switched_to": profile_name }))
}

fn sso_logout_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile_name = params["profile"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: profile"))?
        .to_string();
    let config_path = aws_config_path();
    let content = std::fs::read_to_string(&config_path)?;
    let profiles = parse_sso_profiles(&content)?;
    let profile = profiles
        .into_iter()
        .find(|p| p.name == profile_name)
        .ok_or_else(|| anyhow::anyhow!("profile not found: {profile_name}"))?;
    clear_sso_token(&profile.sso_start_url)?;
    let mut s = state.lock().unwrap();
    if s.active_profile
        .as_ref()
        .map(|p| p.name == profile_name)
        .unwrap_or(false)
    {
        s.active_profile = None;
    }
    Ok(json!({ "logged_out": profile_name }))
}

async fn sso_login_handler(state: SharedState, profile_name: &str) -> Result<Value> {
    let config_path = aws_config_path();
    let content = std::fs::read_to_string(&config_path)?;
    let profiles = parse_sso_profiles(&content)?;
    let profile = profiles
        .into_iter()
        .find(|p| p.name == profile_name)
        .ok_or_else(|| anyhow::anyhow!("profile not found: {profile_name}"))?;
    sso_login(&profile).await?;
    state.lock().unwrap().active_profile = Some(profile.clone());
    Ok(json!({
        "logged_in": profile_name,
        "account_id": profile.sso_account_id,
        "role": profile.sso_role_name,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::config::parse_sso_profiles;

    #[test]
    fn list_accounts_parse_empty_config() {
        let content = "";
        let profiles = parse_sso_profiles(content).unwrap();
        assert_eq!(profiles.len(), 0);
    }
}
