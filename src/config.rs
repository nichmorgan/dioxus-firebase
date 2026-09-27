//! Firebase Auth configuration.
//!
//! The **app** owns `api_key`, `app_id`, `project_id`, and related Firebase
//! Options — never hardcode them in this crate.

use crate::error::FirebaseError;

/// Firebase project options supplied by the host app.
///
/// Mirrors the fields commonly passed to `FirebaseOptions` / `FirebaseApp`
/// initialization. Staging vs production selection belongs in the app
/// (for example via `cfg!(debug_assertions)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirebaseOptions {
    /// Web API key (`apiKey`).
    pub api_key: String,
    /// Mobile / web app id (`appId` / `google_app_id`).
    pub app_id: String,
    /// GCP / Firebase project id.
    pub project_id: String,
    /// Optional GCM / FCM sender id.
    pub messaging_sender_id: Option<String>,
    /// Optional Cloud Storage bucket.
    pub storage_bucket: Option<String>,
    /// Optional Realtime Database URL.
    pub database_url: Option<String>,
}

impl FirebaseOptions {
    pub(crate) fn validate(&self) -> Result<(), FirebaseError> {
        if self.api_key.trim().is_empty() {
            return Err(FirebaseError::InvalidConfig(
                "api_key must not be empty".into(),
            ));
        }
        if self.app_id.trim().is_empty() {
            return Err(FirebaseError::InvalidConfig(
                "app_id must not be empty".into(),
            ));
        }
        if self.project_id.trim().is_empty() {
            return Err(FirebaseError::InvalidConfig(
                "project_id must not be empty".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn valid_options() -> FirebaseOptions {
        FirebaseOptions {
            api_key: "AIzaSyTestKey".into(),
            app_id: "1:123456789:android:abcdef".into(),
            project_id: "demo-project".into(),
            messaging_sender_id: Some("123456789".into()),
            storage_bucket: Some("demo-project.appspot.com".into()),
            database_url: Some("https://demo-project.firebaseio.com".into()),
        }
    }

    #[rstest]
    fn test_valid_options_pass(valid_options: FirebaseOptions) {
        assert_eq!(valid_options.validate(), Ok(()));
    }

    #[rstest]
    fn test_valid_options_minimal_pass(mut valid_options: FirebaseOptions) {
        valid_options.messaging_sender_id = None;
        valid_options.storage_bucket = None;
        valid_options.database_url = None;
        assert_eq!(valid_options.validate(), Ok(()));
    }

    #[rstest]
    #[case("", "app", "project", "api_key must not be empty")]
    #[case("  ", "app", "project", "api_key must not be empty")]
    #[case("key", "", "project", "app_id must not be empty")]
    #[case("key", "  ", "project", "app_id must not be empty")]
    #[case("key", "app", "", "project_id must not be empty")]
    #[case("key", "app", "   ", "project_id must not be empty")]
    fn test_validation_failures(
        #[case] api_key: &str,
        #[case] app_id: &str,
        #[case] project_id: &str,
        #[case] expected_msg: &str,
    ) {
        let options = FirebaseOptions {
            api_key: api_key.into(),
            app_id: app_id.into(),
            project_id: project_id.into(),
            messaging_sender_id: None,
            storage_bucket: None,
            database_url: None,
        };
        let err = options.validate().unwrap_err();
        match err {
            FirebaseError::InvalidConfig(msg) => assert!(msg.contains(expected_msg)),
            _ => panic!("expected InvalidConfig, got {err:?}"),
        }
    }
}
