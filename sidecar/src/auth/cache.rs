use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCredentials {
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "secretAccessKey")]
    pub secret_access_key: String,
    #[serde(rename = "sessionToken")]
    pub session_token: String,
    #[serde(rename = "expiration")]
    pub expiration: DateTime<Utc>,
}

impl CachedCredentials {
    pub fn is_expired(&self) -> bool {
        self.expiration <= Utc::now()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoTokenCache {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
    #[serde(rename = "region")]
    pub region: String,
    #[serde(rename = "startUrl")]
    pub start_url: String,
}

impl SsoTokenCache {
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }
}

pub fn sso_cache_dir() -> PathBuf {
    dirs::home_dir()
        .expect("home dir not found")
        .join(".aws")
        .join("sso")
        .join("cache")
}

pub fn cache_key(start_url: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(start_url.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn read_sso_token(start_url: &str) -> Result<Option<SsoTokenCache>> {
    let key = cache_key(start_url);
    let path = sso_cache_dir().join(format!("{key}.json"));
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading cache file {}", path.display()))?;
    let token: SsoTokenCache = serde_json::from_str(&content)
        .with_context(|| "parsing SSO token cache")?;
    Ok(Some(token))
}

pub fn write_sso_token(token: &SsoTokenCache) -> Result<()> {
    let key = cache_key(&token.start_url);
    let dir = sso_cache_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{key}.json"));
    let content = serde_json::to_string_pretty(token)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn clear_sso_token(start_url: &str) -> Result<()> {
    let key = cache_key(start_url);
    let path = sso_cache_dir().join(format!("{key}.json"));
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_is_deterministic() {
        let k1 = cache_key("https://my-org.awsapps.com/start");
        let k2 = cache_key("https://my-org.awsapps.com/start");
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 64);
    }

    #[test]
    fn cache_key_differs_for_different_urls() {
        let k1 = cache_key("https://a.com/start");
        let k2 = cache_key("https://b.com/start");
        assert_ne!(k1, k2);
    }

    #[test]
    fn credentials_expired_when_past() {
        let creds = CachedCredentials {
            access_key_id: "A".into(),
            secret_access_key: "S".into(),
            session_token: "T".into(),
            expiration: Utc::now() - chrono::Duration::hours(1),
        };
        assert!(creds.is_expired());
    }

    #[test]
    fn credentials_valid_when_future() {
        let creds = CachedCredentials {
            access_key_id: "A".into(),
            secret_access_key: "S".into(),
            session_token: "T".into(),
            expiration: Utc::now() + chrono::Duration::hours(1),
        };
        assert!(!creds.is_expired());
    }
}
