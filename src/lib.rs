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
//! JusFine dogfoods this crate for email/password auth.

mod auth;
mod config;
mod error;
mod host_protocol;
mod init;
mod native;
mod subscribe;
mod user;

pub use auth::{
    create_user_with_email, current_user, id_token, send_password_reset_email, sign_in_with_email,
    sign_out, update_display_name, use_emulator,
};
pub use config::FirebaseOptions;
pub use error::FirebaseError;
pub use init::{initialize, is_initialized};
pub use subscribe::{subscribe_auth_state, AuthStateSubscription};
pub use user::User;

/// Convenient re-exports for application crates.
pub mod prelude {
    pub use crate::{
        create_user_with_email, current_user, id_token, initialize, is_initialized,
        send_password_reset_email, sign_in_with_email, sign_out, subscribe_auth_state,
        update_display_name, use_emulator, AuthStateSubscription, FirebaseError, FirebaseOptions,
        User,
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
        };
        let err = opts.validate().unwrap_err();
        assert!(err.to_string().contains(expected_err));
    }
}
