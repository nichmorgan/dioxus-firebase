//! iOS E2E integration test target.
#![cfg(target_os = "ios")]

use super::common::{demo_options, unique_email};
use dioxus_firebase::prelude::*;
use rstest::{fixture, rstest};
use std::sync::mpsc;
use std::time::Duration;

#[fixture]
fn emulator_endpoint() -> (String, u16) {
    let host = std::env::var("FIREBASE_AUTH_EMULATOR_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("FIREBASE_AUTH_EMULATOR_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9099);
    (host, port)
}

#[rstest]
fn test_ios_firebase_auth_emulator_lifecycle(
    demo_options: FirebaseOptions,
    unique_email: String,
    emulator_endpoint: (String, u16),
) -> Result<(), FirebaseError> {
    // 1. Initialize
    initialize_app(demo_options)?;

    // 2. Connect to Local Emulator
    let (host, port) = emulator_endpoint;
    use_auth_emulator(&host, port)?;

    // 3. Subscribe to Auth State
    let (tx, rx) = mpsc::channel();
    let _sub = auth_state_changes(move |user| {
        let _ = tx.send(user);
    })?;

    // 4. Create User
    let password = "TestPassword123!";
    let created = create_user_with_email_and_password(&unique_email, password)?;
    assert_eq!(created.email.as_deref(), Some(unique_email.as_str()));
    assert!(
        created
            .provider_data
            .iter()
            .any(|p| p.provider_id == "password"),
        "email user should list password provider: {:?}",
        created.provider_data
    );

    // 5. Update Profile
    update_display_name("iOS E2E User")?;
    let updated = current_user()?.expect("should have updated user");
    assert_eq!(updated.display_name.as_deref(), Some("iOS E2E User"));
    assert!(updated
        .provider_data
        .iter()
        .any(|p| p.provider_id == "password"));

    // 6. Sign Out & Sign Back In
    sign_out()?;
    assert!(current_user()?.is_none());

    let signed_in = sign_in_with_email_and_password(&unique_email, password)?;
    assert_eq!(signed_in.uid, created.uid);
    assert!(signed_in
        .provider_data
        .iter()
        .any(|p| p.provider_id == "password"));

    sign_out()?;
    Ok(())
}
