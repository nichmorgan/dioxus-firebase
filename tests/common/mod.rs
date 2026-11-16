//! Shared test fixtures across E2E integration test targets.

use dioxus_firebase::prelude::*;
use rstest::fixture;
use std::time::{SystemTime, UNIX_EPOCH};

/// Baseline Firebase options for emulator / test execution.
#[fixture]
pub fn demo_options() -> FirebaseOptions {
    FirebaseOptions {
        api_key: "demo-api-key".into(),
        app_id: if cfg!(target_os = "ios") {
            "1:123456789:ios:0123456789abcdef"
        } else {
            "1:123456789:android:demo"
        }
        .into(),
        project_id: "demo-e2e-project".into(),
        messaging_sender_id: None,
        storage_bucket: None,
        database_url: None,
        google_web_client_id: None,
    }
}

/// Generates a timestamped unique user email address.
#[fixture]
pub fn unique_email() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("e2e-user-{millis}@example.com")
}
