use anyhow::Result;
use serde_json::{json, Value};

use crate::auth::state::SharedState;
use crate::mcp::dispatcher::Dispatcher;
use crate::services::s3 as svc;

pub fn register(dispatcher: &mut Dispatcher, state: SharedState) {
    let s1 = state.clone();
    dispatcher.register("s3_list_buckets", Box::new(move |_params| {
        let state = s1.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                s3_list_buckets_handler(state).await
            })
        })
    }));

    let s2 = state.clone();
    dispatcher.register("s3_list_objects", Box::new(move |params| {
        let state = s2.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                s3_list_objects_handler(state, params).await
            })
        })
    }));

    let s3 = state.clone();
    dispatcher.register("s3_get_object", Box::new(move |params| {
        let state = s3.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                s3_get_object_handler(state, params).await
            })
        })
    }));

    let s4 = state.clone();
    dispatcher.register("s3_put_object", Box::new(move |params| {
        let state = s4.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                s3_put_object_handler(state, params).await
            })
        })
    }));

    let s5 = state.clone();
    dispatcher.register("s3_delete_object", Box::new(move |params| {
        let state = s5.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                s3_delete_object_handler(state, params).await
            })
        })
    }));

    let s6 = state.clone();
    dispatcher.register("s3_presign", Box::new(move |params| {
        let state = s6.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                s3_presign_handler(state, params).await
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

async fn s3_list_buckets_handler(state: SharedState) -> Result<Value> {
    let profile = active_profile(&state)?;
    let client = svc::build_client(&profile).await?;
    let buckets = svc::list_buckets(&client).await?;
    Ok(json!({ "buckets": buckets }))
}

async fn s3_list_objects_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let bucket = params["bucket"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: bucket"))?;
    let prefix = params["prefix"].as_str();
    let max_keys = params["max_keys"].as_i64().map(|v| v as i32);
    let client = svc::build_client_for_bucket(&profile, bucket).await?;
    let objects = svc::list_objects(&client, bucket, prefix, max_keys).await?;
    Ok(json!({ "objects": objects }))
}

async fn s3_get_object_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let bucket = params["bucket"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: bucket"))?;
    let key = params["key"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: key"))?;
    let client = svc::build_client_for_bucket(&profile, bucket).await?;
    let bytes = svc::get_object(&client, bucket, key).await?;
    let content = match String::from_utf8(bytes.clone()) {
        Ok(s) => json!({ "content": s, "encoding": "utf8" }),
        Err(_) => json!({ "content": base64_encode(&bytes), "encoding": "base64" }),
    };
    Ok(content)
}

async fn s3_put_object_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let bucket = params["bucket"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: bucket"))?;
    let key = params["key"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: key"))?;
    let body = params["body"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: body"))?
        .as_bytes()
        .to_vec();
    let client = svc::build_client_for_bucket(&profile, bucket).await?;
    svc::put_object(&client, bucket, key, body).await?;
    Ok(json!({ "uploaded": key, "bucket": bucket }))
}

async fn s3_delete_object_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let bucket = params["bucket"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: bucket"))?;
    let key = params["key"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: key"))?;
    let client = svc::build_client_for_bucket(&profile, bucket).await?;
    svc::delete_object(&client, bucket, key).await?;
    Ok(json!({ "deleted": key, "bucket": bucket }))
}

async fn s3_presign_handler(state: SharedState, params: Value) -> Result<Value> {
    let profile = active_profile(&state)?;
    let bucket = params["bucket"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: bucket"))?;
    let key = params["key"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required param: key"))?;
    let expires_secs = params["expires_secs"].as_u64().unwrap_or(3600);
    let client = svc::build_client_for_bucket(&profile, bucket).await?;
    let url = svc::presign_get(&client, bucket, key, expires_secs).await?;
    Ok(json!({ "url": url, "expires_secs": expires_secs }))
}

fn base64_encode(bytes: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };
        out.push(CHARS[(b0 >> 2)] as char);
        out.push(CHARS[((b0 & 3) << 4) | (b1 >> 4)] as char);
        if chunk.len() > 1 {
            out.push(CHARS[((b1 & 0xf) << 2) | (b2 >> 6)] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARS[b2 & 0x3f] as char);
        } else {
            out.push('=');
        }
    }
    out
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
    fn missing_bucket_param_errors() {
        let params = json!({});
        let result: Result<&str> = params["bucket"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing required param: bucket"));
        assert!(result.unwrap_err().to_string().contains("bucket"));
    }

    #[test]
    fn base64_encode_hello() {
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
    }

    #[test]
    fn base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }
}
