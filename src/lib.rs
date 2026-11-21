//! Unofficial Firebase Auth primitives for Dioxus mobile.
//!
//! **Mobile only** (iOS and Android). Web and desktop return
//! [`FirebaseError::UnsupportedPlatform`].
//!
//! Native host: JNI on Android, a small ObjC-visible Swift shim on iOS — not UniFFI.
//!
//! The app owns [`FirebaseOptions`] (api key, app id, project id, …). This crate
//! never hardcodes Firebase keys. Auth failures return **wire codes**
//! (`invalid-credential`, `email-already-in-use`, …) — map them to PT-BR (or
//! other) copy in the app.
//!
//! The public API follows FlutterFire `firebase_auth`. The repository README
//! maps each call to the Dart name, lists what is not wrapped, and describes
//! the extras. Host names and platform notes are in `docs/flutterfire.md`.

mod auth;
mod config;
mod error;
mod host_protocol;
mod init;
mod native;
mod subscribe;
#[cfg(test)]
mod test_support;
mod user;

pub use auth::{
    create_user_with_email_and_password, current_user, delete_current_user, get_id_token,
    link_with_apple, link_with_google, pick_apple_credential, pick_google_credential,
    reload_current_user, send_password_reset_email, sign_in_with_apple,
    sign_in_with_email_and_password, sign_in_with_google, sign_out, unlink, update_display_name,
    update_photo_url, use_auth_emulator,
};
pub use config::FirebaseOptions;
pub use error::FirebaseError;
pub use init::{initialize_app, is_initialized};
pub use subscribe::{auth_state_changes, AuthStateSubscription};
pub use user::{ProviderCredential, User, UserInfo};

/// Convenient re-exports for application crates.
pub mod prelude {
    pub use crate::{
        auth_state_changes, create_user_with_email_and_password, current_user, delete_current_user,
        get_id_token, initialize_app, is_initialized, link_with_apple, link_with_google,
        pick_apple_credential, pick_google_credential, reload_current_user,
        send_password_reset_email, sign_in_with_apple, sign_in_with_email_and_password,
        sign_in_with_google, sign_out, unlink, update_display_name, update_photo_url,
        use_auth_emulator, AuthStateSubscription, FirebaseError, FirebaseOptions,
        ProviderCredential, User, UserInfo,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn sample_options() -> FirebaseOptions {
        FirebaseOptions {
            api_key: "valid-key".into(),
            app_id: "valid-app-id".into(),
            project_id: "valid-project".into(),
            messaging_sender_id: None,
            storage_bucket: None,
            database_url: None,
            google_web_client_id: None,
        }
    }

    #[rstest]
    #[case("", "app", "proj", "api_key must not be empty")]
    #[case("key", "  ", "proj", "app_id must not be empty")]
    #[case("key", "app", "", "project_id must not be empty")]
    fn test_invalid_options(
        #[case] api_key: &str,
        #[case] app_id: &str,
        #[case] project_id: &str,
        #[case] expected_err: &str,
    ) {
        let opts = FirebaseOptions {
            api_key: api_key.into(),
            app_id: app_id.into(),
            project_id: project_id.into(),
            messaging_sender_id: None,
            storage_bucket: None,
            database_url: None,
            google_web_client_id: None,
        };
        let err = opts.validate().unwrap_err();
        assert!(err.to_string().contains(expected_err));
    }
}
