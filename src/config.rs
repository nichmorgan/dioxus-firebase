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
