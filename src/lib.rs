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
    use std::sync::Mutex;

    static INIT_SLOT: Mutex<()> = Mutex::new(());

    fn sample_options() -> FirebaseOptions {
        FirebaseOptions {
            api_key: "test-api-key".into(),
            app_id: "1:123:android:abc".into(),
            project_id: "demo-project".into(),
            messaging_sender_id: None,
            storage_bucket: None,
            database_url: None,
        }
    }

    fn with_init_slot<R>(f: impl FnOnce() -> R) -> R {
        let _guard = INIT_SLOT.lock().unwrap_or_else(|e| e.into_inner());
        init::reset_initialized_for_test();
        f()
    }

    #[cfg(not(target_os = "ios"))]
    fn assert_all_primitives_err(pred: impl Fn(&FirebaseError) -> bool) {
        let check = |op: &str, err: FirebaseError| {
            assert!(pred(&err), "{op}: {err:?}");
        };
        check(
            "sign_in_with_email",
            sign_in_with_email("a@b.c", "secret").unwrap_err(),
        );
        check(
            "create_user_with_email",
            create_user_with_email("a@b.c", "secret").unwrap_err(),
        );
        check(
            "update_display_name",
            update_display_name("Ada").unwrap_err(),
        );
        check(
            "send_password_reset_email",
            send_password_reset_email("a@b.c").unwrap_err(),
        );
        check("sign_out", sign_out().unwrap_err());
        check("current_user", current_user().unwrap_err());
        check("id_token", id_token(false).unwrap_err());
        check("use_emulator", use_emulator("10.0.2.2", 9099).unwrap_err());
        check(
            "subscribe_auth_state",
            subscribe_auth_state(|_| {}).unwrap_err(),
        );
    }

    #[test]
    fn rejects_empty_api_key() {
        with_init_slot(|| {
            let err = initialize(FirebaseOptions {
                api_key: "  ".into(),
                ..sample_options()
            })
            .expect_err("empty api_key must fail");
            assert!(matches!(err, FirebaseError::InvalidConfig(_)));
            assert!(!is_initialized());
        });
    }

    #[test]
    fn rejects_empty_app_id_and_project_id() {
        with_init_slot(|| {
            let err = initialize(FirebaseOptions {
                app_id: "".into(),
                ..sample_options()
            })
            .expect_err("empty app_id must fail");
            assert!(matches!(err, FirebaseError::InvalidConfig(_)));

            init::reset_initialized_for_test();
            let err = initialize(FirebaseOptions {
                project_id: " ".into(),
                ..sample_options()
            })
            .expect_err("empty project_id must fail");
            assert!(matches!(err, FirebaseError::InvalidConfig(_)));
        });
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    #[test]
    fn unsupported_on_non_mobile() {
        with_init_slot(|| {
            let err = initialize(sample_options()).expect_err("desktop/web must fail");
            assert_eq!(err, FirebaseError::UnsupportedPlatform);
            assert!(!is_initialized());
        });
    }

    #[cfg(target_os = "android")]
    #[test]
    fn initialize_reports_host_missing_without_android_context() {
        with_init_slot(|| {
            let err = initialize(sample_options()).expect_err("no android context");
            assert!(matches!(err, FirebaseError::HostMissing(_)));
            assert!(!is_initialized());
        });
    }

    #[test]
    fn double_init_is_rejected() {
        with_init_slot(|| {
            init::mark_initialized_for_test();
            let err = initialize(sample_options()).expect_err("second init must fail");
            assert_eq!(err, FirebaseError::AlreadyInitialized);
        });
    }

    #[test]
    fn sign_in_rejects_empty_email() {
        with_init_slot(|| {
            init::mark_initialized_for_test();
            let err = sign_in_with_email("  ", "secret").expect_err("empty email must fail");
            assert!(matches!(err, FirebaseError::InvalidConfig(_)));
        });
    }

    #[test]
    fn create_user_rejects_empty_password() {
        with_init_slot(|| {
            init::mark_initialized_for_test();
            let err = create_user_with_email("a@b.c", "").expect_err("empty password must fail");
            assert!(matches!(err, FirebaseError::InvalidConfig(_)));
        });
    }

    #[test]
    fn reset_rejects_empty_email() {
        with_init_slot(|| {
            init::mark_initialized_for_test();
            let err = send_password_reset_email("").expect_err("empty email must fail");
            assert!(matches!(err, FirebaseError::InvalidConfig(_)));
        });
    }

    #[test]
    fn auth_primitives_require_init() {
        with_init_slot(|| {
            assert_eq!(
                sign_in_with_email("a@b.c", "x").unwrap_err(),
                FirebaseError::NotInitialized
            );
            assert_eq!(current_user().unwrap_err(), FirebaseError::NotInitialized);
            assert_eq!(
                subscribe_auth_state(|_| {}).unwrap_err(),
                FirebaseError::NotInitialized
            );
        });
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    #[test]
    fn auth_primitives_unsupported_on_non_mobile() {
        with_init_slot(|| {
            init::mark_initialized_for_test();
            assert_all_primitives_err(|e| *e == FirebaseError::UnsupportedPlatform);
        });
    }

    #[cfg(target_os = "android")]
    #[test]
    fn auth_primitives_report_host_missing_without_android_context() {
        with_init_slot(|| {
            init::mark_initialized_for_test();
            assert_all_primitives_err(
                |e| matches!(e, FirebaseError::HostMissing(msg) if msg.contains("ndk_context")),
            );
        });
    }
}
