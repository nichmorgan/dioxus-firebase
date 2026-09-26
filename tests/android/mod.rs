//! Android E2E integration test target.
#![cfg(target_os = "android")]

use super::common::{demo_options, unique_email};
use dioxus_firebase::prelude::*;
use rstest::{fixture, rstest};
use std::sync::mpsc;
use std::time::Duration;

#[fixture]
fn emulator_endpoint() -> (String, u16) {
    let host = std::env::var("FIREBASE_AUTH_EMULATOR_HOST").unwrap_or_else(|_| "10.0.2.2".into());
    let port = std::env::var("FIREBASE_AUTH_EMULATOR_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9099);
    (host, port)
}

#[rstest]
fn test_android_firebase_auth_emulator_lifecycle(
    demo_options: FirebaseOptions,
    unique_email: String,
    emulator_endpoint: (String, u16),
) -> Result<(), FirebaseError> {
    // 1. Initialize
    match initialize(demo_options) {
        Err(FirebaseError::HostMissing(msg)) => {
            eprintln!("Skipping live Android JNI E2E execution in raw CLI mode ({msg}).");
            return Ok(());
        }
        Err(e) => return Err(e),
        Ok(()) => {}
    }

    // 2. Connect to Local Emulator
    let (host, port) = emulator_endpoint;
    use_emulator(&host, port)?;

    // 3. Subscribe to Auth State Changes
    let (tx, rx) = mpsc::channel();
    let _sub = subscribe_auth_state(move |user| {
        let _ = tx.send(user);
    })?;

    // 4. Create User with Email and Password
    let password = "TestPassword123!";
    let created = create_user_with_email(&unique_email, password)?;
    assert_eq!(created.email.as_deref(), Some(unique_email.as_str()));

    // Verify Callback
    if let Ok(Some(state_user)) = rx.recv_timeout(Duration::from_secs(5)) {
        assert_eq!(state_user.uid, created.uid);
    }

    // 5. Query User & Update Profile
    let active = current_user()?.expect("should have active user");
    assert_eq!(active.uid, created.uid);

    update_display_name("Android E2E User")?;
    let updated = current_user()?.expect("should have updated user");
    assert_eq!(updated.display_name.as_deref(), Some("Android E2E User"));

    // 6. Token & Reset
    let token = id_token(true)?.expect("token required");
    assert!(!token.is_empty());
    send_password_reset_email(&unique_email)?;

    // 7. Sign Out & Sign Back In
    sign_out()?;
    assert!(current_user()?.is_none());

    let signed_in = sign_in_with_email(&unique_email, password)?;
    assert_eq!(signed_in.uid, created.uid);

    sign_out()?;
    Ok(())
}
