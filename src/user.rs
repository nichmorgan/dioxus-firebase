//! Signed-in user snapshot (official Auth `User` / `UserInfo` shape).

/// Linked identity row (`UserInfo` / `providerData` entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserInfo {
    /// Federated provider id (e.g. `password`, `google.com`, `apple.com`, `firebase`).
    pub provider_id: String,
    /// Provider-scoped uid, when present.
    pub uid: Option<String>,
    /// Provider email, when present.
    pub email: Option<String>,
    /// Provider display name, when present.
    pub display_name: Option<String>,
    /// Provider photo URL (`photoURL`), when present.
    pub photo_url: Option<String>,
}

/// Firebase Auth user fields exposed to the app (top-level + `providerData`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// Stable Firebase uid.
    pub uid: String,
    /// Top-level account email, when present.
    pub email: Option<String>,
    /// Top-level display name, when present.
    pub display_name: Option<String>,
    /// Top-level profile photo URL (`photoURL`), when present.
    pub photo_url: Option<String>,
    /// Full linked `providerData` rows (includes `firebase` when the SDK returns it).
    pub provider_data: Vec<UserInfo>,
}

impl User {
    #[allow(dead_code)] // used from native host reply parsing
    pub(crate) fn from_host_fields(
        uid: String,
        email: String,
        display_name: String,
        photo_url: String,
        provider_data: Vec<UserInfo>,
    ) -> Self {
        Self {
            uid,
            email: nonempty(email),
            display_name: nonempty(display_name),
            photo_url: nonempty(photo_url),
            provider_data,
        }
    }
}

/// An ID token from the Google or Apple sheet that was not signed in or linked.
///
/// Send it to a backend that verifies it and links that identity with the
/// Admin SDK; the client never holds two logins for one provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCredential {
    /// Provider-issued OpenID Connect ID token.
    pub id_token: String,
    /// Apple only: the nonce whose SHA-256 hex is the token's `nonce` claim.
    pub raw_nonce: Option<String>,
}

pub(crate) fn nonempty(value: String) -> Option<String> {
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
    use crate::test_support::{sample_provider, sample_user};
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
        "uid123",
        "user@example.com",
        "John Doe",
        "",
        vec![sample_provider("password", Some("uid123"), Some("user@example.com"), Some("John Doe"), None)],
        sample_user(
            "uid123",
            Some("user@example.com"),
            Some("John Doe"),
            None,
            vec![sample_provider("password", Some("uid123"), Some("user@example.com"), Some("John Doe"), None)],
        )
    )]
    #[case(
        "uid123",
        "",
        "  ",
        "  ",
        vec![],
        sample_user("uid123", None, None, None, vec![])
    )]
    #[case(
        "uid123",
        "a@b.c",
        "Ada",
        "https://ex/p.png",
        vec![
            sample_provider("password", Some("uid123"), Some("a@b.c"), Some("Ada"), None),
            sample_provider("google.com", Some("g-uid"), Some("g@ex.com"), Some("G"), Some("https://g/p.png")),
        ],
        sample_user(
            "uid123",
            Some("a@b.c"),
            Some("Ada"),
            Some("https://ex/p.png"),
            vec![
                sample_provider("password", Some("uid123"), Some("a@b.c"), Some("Ada"), None),
                sample_provider("google.com", Some("g-uid"), Some("g@ex.com"), Some("G"), Some("https://g/p.png")),
            ],
        )
    )]
    fn test_user_from_host_fields(
        #[case] uid: &str,
        #[case] email: &str,
        #[case] display_name: &str,
        #[case] photo_url: &str,
        #[case] provider_data: Vec<UserInfo>,
        #[case] expected: User,
    ) {
        let user = User::from_host_fields(
            uid.into(),
            email.into(),
            display_name.into(),
            photo_url.into(),
            provider_data,
        );
        assert_eq!(user, expected);
    }
}
