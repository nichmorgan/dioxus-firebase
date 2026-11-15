//! Profile field preference for auth-state / OK-user wire.
//!
//! Hosts (Kotlin / Swift `userFields`) must keep the same rules in sync.
//! Unit-tested with plain mocks — no Firebase SDK.

#![allow(dead_code)] // exercised by hosts via mirrored logic; tested here

pub(crate) const PROVIDER_GOOGLE: &str = "google.com";
pub(crate) const PROVIDER_APPLE: &str = "apple.com";

/// Mock IdP row (mirrors Firebase `UserInfo` fields we care about).
#[derive(Debug, Clone, Copy)]
pub(crate) struct MockProvider {
    pub id: &'static str,
    pub email: &'static str,
    pub name: &'static str,
    pub photo: &'static str,
}

/// Mock top-level Firebase user profile.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MockTopLevel {
    pub email: &'static str,
    pub name: &'static str,
    pub photo: &'static str,
}

/// Preferred `(email, display_name, photo_url)` after trim / blank fallthrough.
///
/// - **email** / **photo**: google.com → apple.com → top-level
/// - **display_name**: top-level → google.com → apple.com
pub(crate) fn prefer_profile_fields(
    top: MockTopLevel,
    providers: &[MockProvider],
) -> (String, String, String) {
    let google = providers.iter().find(|p| p.id == PROVIDER_GOOGLE);
    let apple = providers.iter().find(|p| p.id == PROVIDER_APPLE);

    let email = pick_non_blank(&[
        google.map(|p| p.email).unwrap_or(""),
        apple.map(|p| p.email).unwrap_or(""),
        top.email,
    ]);
    let photo = pick_non_blank(&[
        google.map(|p| p.photo).unwrap_or(""),
        apple.map(|p| p.photo).unwrap_or(""),
        top.photo,
    ]);
    let name = pick_non_blank(&[
        top.name,
        google.map(|p| p.name).unwrap_or(""),
        apple.map(|p| p.name).unwrap_or(""),
    ]);
    (email, name, photo)
}

fn pick_non_blank(candidates: &[&str]) -> String {
    for c in candidates {
        let t = c.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(
        MockTopLevel { email: "pw@ex.com", name: "Pw Name", photo: "" },
        &[MockProvider { id: "password", email: "pw@ex.com", name: "Pw Name", photo: "" }],
        ("pw@ex.com", "Pw Name", "")
    )]
    #[case(
        MockTopLevel { email: "pw@ex.com", name: "App Name", photo: "https://app/p.png" },
        &[
            MockProvider { id: "password", email: "pw@ex.com", name: "Pw", photo: "" },
            MockProvider {
                id: PROVIDER_GOOGLE,
                email: "g@ex.com",
                name: "Google Name",
                photo: "https://g/p.png",
            },
        ],
        ("g@ex.com", "App Name", "https://g/p.png")
    )]
    #[case(
        MockTopLevel { email: "g@ex.com", name: "", photo: "" },
        &[MockProvider {
            id: PROVIDER_GOOGLE,
            email: "g@ex.com",
            name: "Google Name",
            photo: "https://g/p.png",
        }],
        ("g@ex.com", "Google Name", "https://g/p.png")
    )]
    #[case(
        MockTopLevel { email: "pw@ex.com", name: "App", photo: "" },
        &[
            MockProvider {
                id: PROVIDER_GOOGLE,
                email: "",
                name: "G",
                photo: "",
            },
            MockProvider {
                id: PROVIDER_APPLE,
                email: "a@ex.com",
                name: "Apple",
                photo: "https://a/p.png",
            },
        ],
        ("a@ex.com", "App", "https://a/p.png")
    )]
    #[case(
        MockTopLevel { email: "", name: "", photo: "" },
        &[],
        ("", "", "")
    )]
    #[case(
        MockTopLevel { email: "  ", name: "  Top  ", photo: "" },
        &[MockProvider {
            id: PROVIDER_GOOGLE,
            email: "  g@ex.com  ",
            name: "  G  ",
            photo: "  https://g  ",
        }],
        ("g@ex.com", "Top", "https://g")
    )]
    fn test_prefer_profile_fields(
        #[case] top: MockTopLevel,
        #[case] providers: &[MockProvider],
        #[case] expected: (&str, &str, &str),
    ) {
        let (email, name, photo) = prefer_profile_fields(top, providers);
        assert_eq!((email.as_str(), name.as_str(), photo.as_str()), expected);
    }
}
