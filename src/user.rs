//! Signed-in user snapshot.

/// Firebase Auth user fields exposed to the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// Stable Firebase uid.
    pub uid: String,
    /// Account email, when present.
    pub email: Option<String>,
    /// Display name, when present.
    pub display_name: Option<String>,
}

impl User {
    #[allow(dead_code)] // used from native host reply parsing
    pub(crate) fn from_host_fields(uid: String, email: String, display_name: String) -> Self {
        Self {
            uid,
            email: nonempty(email),
            display_name: nonempty(display_name),
        }
    }
}

#[allow(dead_code)]
fn nonempty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
