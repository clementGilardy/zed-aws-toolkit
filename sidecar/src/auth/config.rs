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

// Parses all sections from an AWS config file into a map of section_header -> key/value pairs.
// Handles both legacy format (sso_start_url directly in [profile X]) and
// SSO v2 format ([profile X] references [sso-session Y] for start_url/region).
fn parse_sections(content: &str) -> HashMap<String, HashMap<String, String>> {
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_name: Option<String> = None;
    let mut current: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(name) = current_name.take() {
                sections.insert(name, current.clone());
            }
            current.clear();
            current_name = Some(line[1..line.len() - 1].trim().to_string());
        } else if let Some((k, v)) = line.split_once('=') {
            current.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    if let Some(name) = current_name {
        sections.insert(name, current);
    }
    sections
}

pub fn parse_sso_profiles(content: &str) -> Result<Vec<SsoProfile>> {
    let sections = parse_sections(content);
    let mut profiles: Vec<SsoProfile> = Vec::new();

    for (header, map) in &sections {
        // Match [profile X] or bare [X] (legacy)
        let profile_name = if let Some(n) = header.strip_prefix("profile ") {
            n.to_string()
        } else if header == "default" {
            header.clone()
        } else {
            continue;
        };

        // Skip sections without sso_account_id — not an SSO profile
        let sso_account_id = match map.get("sso_account_id") {
            Some(v) => v.clone(),
            None => continue,
        };
        let sso_role_name = match map.get("sso_role_name") {
            Some(v) => v.clone(),
            None => continue,
        };
        let region = map.get("region").cloned().unwrap_or_else(|| "us-east-1".to_string());

        // SSO v2: sso_session points to [sso-session X]
        let (sso_start_url, sso_region) = if let Some(session_name) = map.get("sso_session") {
            let session_key = format!("sso-session {}", session_name);
            let session = sections.get(&session_key)
                .with_context(|| format!("sso-session '{}' not found in config", session_name))?;
            let url = session.get("sso_start_url")
                .with_context(|| format!("missing sso_start_url in [sso-session {}]", session_name))?
                .clone();
            let region = session.get("sso_region")
                .with_context(|| format!("missing sso_region in [sso-session {}]", session_name))?
                .clone();
            (url, region)
        } else {
            // Legacy: sso_start_url/sso_region directly in profile
            let url = map.get("sso_start_url").context("missing sso_start_url")?.clone();
            let region = map.get("sso_region").context("missing sso_region")?.clone();
            (url, region)
        };

        profiles.push(SsoProfile {
            name: profile_name,
            sso_start_url,
            sso_region,
            sso_account_id,
            sso_role_name,
            region,
        });
    }
    Ok(profiles)
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

    #[test]
    fn parse_sso_v2_format() {
        let config = r#"
[profile billy-staging]
region = eu-central-1
sso_session = billy-staging
sso_account_id = 783299916304
sso_role_name = AWSAdministratorAccess

[sso-session billy-staging]
sso_start_url = https://d-996711ea90.awsapps.com/start
sso_region = eu-central-1
sso_registration_scopes = sso:account:access
"#;
        let profiles = parse_sso_profiles(config).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "billy-staging");
        assert_eq!(profiles[0].sso_account_id, "783299916304");
        assert_eq!(profiles[0].sso_start_url, "https://d-996711ea90.awsapps.com/start");
        assert_eq!(profiles[0].sso_region, "eu-central-1");
        assert_eq!(profiles[0].sso_role_name, "AWSAdministratorAccess");
    }
}
