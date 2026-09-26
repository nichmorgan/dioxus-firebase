//! Typed errors for Firebase Auth primitives.
//!
//! Auth failures carry Firebase **wire codes** (for example
//! `invalid-credential`, `email-already-in-use`). Apps map those codes to
//! localized copy — this crate never returns PT-BR (or other) user strings.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FirebaseError {
    /// Required Firebase options or request arguments are missing / invalid.
    #[error("invalid Firebase config: {0}")]
    InvalidConfig(String),

    /// [`crate::initialize`] was already called successfully.
    #[error("Firebase is already initialized")]
    AlreadyInitialized,

    /// A primitive was called before [`crate::initialize`].
    #[error("Firebase has not been initialized")]
    NotInitialized,

    /// This crate only supports iOS and Android.
    #[error("unsupported platform: dioxus-firebase is mobile-only (iOS/Android)")]
    UnsupportedPlatform,

    /// Native SDK classes or the iOS host shim are missing from the app.
    #[error("native Firebase host missing: {0}")]
    HostMissing(String),

    /// Firebase Auth failure with a stable wire `code`.
    ///
    /// Codes follow the Firebase Auth client convention
    /// (`invalid-credential`, `email-already-in-use`, `weak-password`, …).
    #[error("Firebase Auth error [{code}]: {message}")]
    Auth { code: String, message: String },

    /// Exception / error forwarded from the native host outside Auth codes.
    #[error("native Firebase error: {message}")]
    Native { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(
        FirebaseError::InvalidConfig("missing api_key".into()),
        "invalid Firebase config: missing api_key"
    )]
    #[case(FirebaseError::AlreadyInitialized, "Firebase is already initialized")]
    #[case(FirebaseError::NotInitialized, "Firebase has not been initialized")]
    #[case(
        FirebaseError::UnsupportedPlatform,
        "unsupported platform: dioxus-firebase is mobile-only (iOS/Android)"
    )]
    #[case(
        FirebaseError::HostMissing("DioxusFirebaseAuthHost not found".into()),
        "native Firebase host missing: DioxusFirebaseAuthHost not found"
    )]
    #[case(
        FirebaseError::Auth {
            code: "invalid-credential".into(),
            message: "The email address is badly formatted.".into(),
        },
        "Firebase Auth error [invalid-credential]: The email address is badly formatted."
    )]
    #[case(
        FirebaseError::Native {
            message: "timed out waiting for main looper".into(),
        },
        "native Firebase error: timed out waiting for main looper"
    )]
    fn test_error_display_formatting(#[case] err: FirebaseError, #[case] expected_display: &str) {
        assert_eq!(err.to_string(), expected_display);
    }

    #[test]
    fn test_error_clone_and_equality() {
        let err1 = FirebaseError::Auth {
            code: "invalid-credential".into(),
            message: "msg".into(),
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }
}
