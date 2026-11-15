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
    /// Profile photo URL, when present.
    pub photo_url: Option<String>,
}

impl User {
    #[allow(dead_code)] // used from native host reply parsing
    pub(crate) fn from_host_fields(
        uid: String,
        email: String,
        display_name: String,
        provider_ids: String,
        photo_url: String,
    ) -> Self {
        Self {
            uid,
            email: nonempty(email),
            display_name: nonempty(display_name),
            provider_ids: parse_provider_ids(provider_ids),
            photo_url: nonempty(photo_url),
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
    use crate::test_support::sample_user;
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
        "uid123", "user@example.com", "John Doe", "password", "",
        sample_user("uid123", "user@example.com", "John Doe", &["password"], "")
    )]
    #[case(
        "uid123", "", "  ", "", "  ",
        sample_user("uid123", "", "", &[], "")
    )]
    #[case(
        "uid123", "a@b.c", "Ada", "password,google.com,apple.com", "https://ex/p.png",
        sample_user(
            "uid123",
            "a@b.c",
            "Ada",
            &["password", "google.com", "apple.com"],
            "https://ex/p.png"
        )
    )]
    fn test_user_from_host_fields(
        #[case] uid: &str,
        #[case] email: &str,
        #[case] display_name: &str,
        #[case] provider_ids: &str,
        #[case] photo_url: &str,
        #[case] expected: User,
    ) {
        let user = User::from_host_fields(
            uid.into(),
            email.into(),
            display_name.into(),
            provider_ids.into(),
            photo_url.into(),
        );
        assert_eq!(user, expected);
    }
}
