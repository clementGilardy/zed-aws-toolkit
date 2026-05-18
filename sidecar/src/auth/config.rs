use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct SsoProfile {
    pub name: String,
    pub sso_start_url: String,
    pub sso_region: String,
    pub sso_account_id: String,
    pub sso_role_name: String,
    pub region: String,
}

pub fn aws_config_path() -> PathBuf {
    dirs::home_dir()
        .expect("home dir not found")
        .join(".aws")
        .join("config")
}

pub fn parse_sso_profiles(content: &str) -> Result<Vec<SsoProfile>> {
    let mut profiles: Vec<SsoProfile> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(name) = current_name.take() {
                if let Ok(p) = build_profile(name, &current) {
                    profiles.push(p);
                }
            }
            current.clear();
            let inner = &line[1..line.len() - 1];
            let name = inner.strip_prefix("profile ").unwrap_or(inner).to_string();
            current_name = Some(name);
        } else if let Some((k, v)) = line.split_once('=') {
            current.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    if let Some(name) = current_name {
        if let Ok(p) = build_profile(name, &current) {
            profiles.push(p);
        }
    }
    Ok(profiles)
}

fn build_profile(name: String, map: &HashMap<String, String>) -> Result<SsoProfile> {
    Ok(SsoProfile {
        name,
        sso_start_url: map.get("sso_start_url").context("missing sso_start_url")?.clone(),
        sso_region: map.get("sso_region").context("missing sso_region")?.clone(),
        sso_account_id: map.get("sso_account_id").context("missing sso_account_id")?.clone(),
        sso_role_name: map.get("sso_role_name").context("missing sso_role_name")?.clone(),
        region: map.get("region").cloned().unwrap_or_else(|| "us-east-1".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CONFIG: &str = r#"
[default]
region = eu-west-1

[profile dev]
sso_start_url = https://my-org.awsapps.com/start
sso_region = eu-west-1
sso_account_id = 123456789012
sso_role_name = DeveloperAccess
region = eu-west-1

[profile prod]
sso_start_url = https://my-org.awsapps.com/start
sso_region = eu-west-1
sso_account_id = 987654321098
sso_role_name = ReadOnlyAccess
region = eu-west-1
"#;

    #[test]
    fn parse_two_sso_profiles() {
        let profiles = parse_sso_profiles(SAMPLE_CONFIG).unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name, "dev");
        assert_eq!(profiles[0].sso_account_id, "123456789012");
        assert_eq!(profiles[1].name, "prod");
        assert_eq!(profiles[1].sso_role_name, "ReadOnlyAccess");
    }

    #[test]
    fn skip_profile_missing_sso_fields() {
        let config = "[default]\nregion = us-east-1\n";
        let profiles = parse_sso_profiles(config).unwrap();
        assert_eq!(profiles.len(), 0);
    }

    #[test]
    fn default_region_fallback() {
        let config = "[profile minimal]\nsso_start_url = https://x\nsso_region = us-east-1\nsso_account_id = 111\nsso_role_name = Admin\n";
        let profiles = parse_sso_profiles(config).unwrap();
        assert_eq!(profiles[0].region, "us-east-1");
    }
}
