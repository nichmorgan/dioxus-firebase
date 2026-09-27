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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("", None)]
    #[case("   ", None)]
    #[case("\t\n", None)]
    #[case("user@example.com", Some("user@example.com".into()))]
    #[case("  user@example.com  ", Some("user@example.com".into()))]
    fn test_nonempty(#[case] input: &str, #[case] expected: Option<String>) {
        assert_eq!(nonempty(input.to_string()), expected);
    }

    #[rstest]
    #[case(
        "uid123", "user@example.com", "John Doe",
        User {
            uid: "uid123".into(),
            email: Some("user@example.com".into()),
            display_name: Some("John Doe".into()),
        }
    )]
    #[case(
        "uid123", "", "  ",
        User {
            uid: "uid123".into(),
            email: None,
            display_name: None,
        }
    )]
    fn test_user_from_host_fields(
        #[case] uid: &str,
        #[case] email: &str,
        #[case] display_name: &str,
        #[case] expected: User,
    ) {
        let user = User::from_host_fields(uid.into(), email.into(), display_name.into());
        assert_eq!(user, expected);
    }
}
