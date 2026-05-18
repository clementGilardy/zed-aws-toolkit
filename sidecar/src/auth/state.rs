use crate::auth::config::SsoProfile;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
pub struct AuthState {
    pub active_profile: Option<SsoProfile>,
}

pub type SharedState = Arc<Mutex<AuthState>>;

pub fn new_shared_state() -> SharedState {
    Arc::new(Mutex::new(AuthState::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::config::SsoProfile;

    fn sample_profile(name: &str) -> SsoProfile {
        SsoProfile {
            name: name.into(),
            sso_start_url: "https://x.awsapps.com/start".into(),
            sso_region: "eu-west-1".into(),
            sso_account_id: "123".into(),
            sso_role_name: "Admin".into(),
            region: "eu-west-1".into(),
        }
    }

    #[test]
    fn default_state_has_no_profile() {
        let state = new_shared_state();
        assert!(state.lock().unwrap().active_profile.is_none());
    }

    #[test]
    fn can_set_active_profile() {
        let state = new_shared_state();
        state.lock().unwrap().active_profile = Some(sample_profile("dev"));
        assert_eq!(state.lock().unwrap().active_profile.as_ref().unwrap().name, "dev");
    }
}
