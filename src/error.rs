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
