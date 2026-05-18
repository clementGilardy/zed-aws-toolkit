use anyhow::{bail, Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_sso::Client as SsoClient;
use aws_sdk_ssooidc::Client as OidcClient;
use chrono::{Duration, Utc};

use crate::auth::cache::{write_sso_token, CachedCredentials, SsoTokenCache};
use crate::auth::config::SsoProfile;

const CLIENT_NAME: &str = "zed-aws-toolkit";
const CLIENT_TYPE: &str = "public";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

pub async fn sso_login(profile: &SsoProfile) -> Result<CachedCredentials> {
    let region = aws_config::meta::region::RegionProviderChain::first_try(
        aws_sdk_ssooidc::config::Region::new(profile.sso_region.clone()),
    );
    let shared_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .no_credentials()
        .load()
        .await;

    let oidc = OidcClient::new(&shared_config);

    let reg = oidc
        .register_client()
        .client_name(CLIENT_NAME)
        .client_type(CLIENT_TYPE)
        .send()
        .await
        .context("register OIDC client")?;

    let client_id = reg.client_id().context("missing client_id")?.to_string();
    let client_secret = reg.client_secret().context("missing client_secret")?.to_string();

    let auth = oidc
        .start_device_authorization()
        .client_id(&client_id)
        .client_secret(&client_secret)
        .start_url(&profile.sso_start_url)
        .send()
        .await
        .context("start device authorization")?;

    let device_code = auth.device_code().context("missing device_code")?.to_string();
    let user_code = auth.user_code().context("missing user_code")?.to_string();
    let verification_uri = auth
        .verification_uri_complete()
        .or_else(|| auth.verification_uri())
        .context("missing verification_uri")?
        .to_string();
    let interval = auth.interval() as u64;

    eprintln!("Opening browser for SSO login...");
    eprintln!("If browser does not open, visit: {verification_uri}");
    eprintln!("User code: {user_code}");
    let _ = open::that(&verification_uri);

    let token = loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(interval.max(5))).await;
        match oidc
            .create_token()
            .client_id(&client_id)
            .client_secret(&client_secret)
            .grant_type(GRANT_TYPE)
            .device_code(&device_code)
            .send()
            .await
        {
            Ok(t) => break t,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("AuthorizationPendingException")
                    || msg.contains("SlowDownException")
                {
                    continue;
                }
                bail!("SSO token exchange failed: {e}");
            }
        }
    };

    let access_token = token.access_token().context("missing access_token")?.to_string();
    let expires_in = token.expires_in();

    let sso_token = SsoTokenCache {
        access_token: access_token.clone(),
        expires_at: Utc::now() + Duration::seconds(expires_in as i64),
        region: profile.sso_region.clone(),
        start_url: profile.sso_start_url.clone(),
    };
    write_sso_token(&sso_token)?;

    let role_creds = get_role_credentials(profile, &access_token).await?;
    Ok(role_creds)
}

pub async fn get_role_credentials(
    profile: &SsoProfile,
    access_token: &str,
) -> Result<CachedCredentials> {
    let region = aws_config::meta::region::RegionProviderChain::first_try(
        aws_sdk_sso::config::Region::new(profile.sso_region.clone()),
    );
    let shared_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .no_credentials()
        .load()
        .await;

    let sso = SsoClient::new(&shared_config);
    let resp = sso
        .get_role_credentials()
        .account_id(&profile.sso_account_id)
        .role_name(&profile.sso_role_name)
        .access_token(access_token)
        .send()
        .await
        .context("get role credentials")?;

    let creds = resp.role_credentials().context("missing role_credentials")?;
    let expiration_ms = creds.expiration();
    let expiration = chrono::DateTime::from_timestamp_millis(expiration_ms)
        .context("invalid expiration timestamp")?;

    Ok(CachedCredentials {
        access_key_id: creds.access_key_id().context("missing access_key_id")?.to_string(),
        secret_access_key: creds
            .secret_access_key()
            .context("missing secret_access_key")?
            .to_string(),
        session_token: creds.session_token().context("missing session_token")?.to_string(),
        expiration,
    })
}
