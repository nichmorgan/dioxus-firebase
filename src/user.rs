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
    /// Linked provider ids (e.g. `password`, `google.com`, `apple.com`).
    pub provider_ids: Vec<String>,
}

impl User {
    #[allow(dead_code)] // used from native host reply parsing
    pub(crate) fn from_host_fields(
        uid: String,
        email: String,
        display_name: String,
        provider_ids: String,
    ) -> Self {
        Self {
            uid,
            email: nonempty(email),
            display_name: nonempty(display_name),
            provider_ids: parse_provider_ids(provider_ids),
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

fn parse_provider_ids(raw: String) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
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
    #[case("", vec![])]
    #[case("password", vec!["password".into()])]
    #[case(
        "password,google.com",
        vec!["password".into(), "google.com".into()]
    )]
    #[case(
        " password , google.com , ",
        vec!["password".into(), "google.com".into()]
    )]
    fn test_parse_provider_ids(#[case] input: &str, #[case] expected: Vec<String>) {
        assert_eq!(parse_provider_ids(input.to_string()), expected);
    }

    #[rstest]
    #[case(
        "uid123", "user@example.com", "John Doe", "password",
        User {
            uid: "uid123".into(),
            email: Some("user@example.com".into()),
            display_name: Some("John Doe".into()),
            provider_ids: vec!["password".into()],
        }
    )]
    #[case(
        "uid123", "", "  ", "",
        User {
            uid: "uid123".into(),
            email: None,
            display_name: None,
            provider_ids: vec![],
        }
    )]
    #[case(
        "uid123", "a@b.c", "Ada", "password,google.com,apple.com",
        User {
            uid: "uid123".into(),
            email: Some("a@b.c".into()),
            display_name: Some("Ada".into()),
            provider_ids: vec!["password".into(), "google.com".into(), "apple.com".into()],
        }
    )]
    fn test_user_from_host_fields(
        #[case] uid: &str,
        #[case] email: &str,
        #[case] display_name: &str,
        #[case] provider_ids: &str,
        #[case] expected: User,
    ) {
        let user = User::from_host_fields(
            uid.into(),
            email.into(),
            display_name.into(),
            provider_ids.into(),
        );
        assert_eq!(user, expected);
    }
}
