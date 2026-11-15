//! Shared host reply protocol (Kotlin / Swift ↔ Rust).
//!
//! Success: `OK` or `OK\x1f…`
//! Auth / native error: `ERR\x1f<code>\x1f<message>`

#![allow(dead_code)] // used from android/ios native modules

use crate::error::FirebaseError;
use crate::user::User;

pub(crate) const UNIT: char = '\u{1f}';

pub(crate) fn parse_void(reply: &str) -> Result<(), FirebaseError> {
    let reply = reply.trim_end_matches('\0');
    if reply == "OK" || reply.is_empty() {
        return Ok(());
    }
    Err(parse_err(reply))
}

pub(crate) fn parse_optional_user(reply: &str) -> Result<Option<User>, FirebaseError> {
    match parse_ok_body(reply)? {
        Some(fields) => Ok(Some(parse_user_fields(fields)?)),
        None => Ok(None),
    }
}

pub(crate) fn parse_optional_string(reply: &str) -> Result<Option<String>, FirebaseError> {
    match parse_ok_body(reply)? {
        Some(value) if value.is_empty() => Ok(None),
        Some(value) => Ok(Some(value.to_string())),
        None => Ok(None),
    }
}

pub(crate) fn parse_user(reply: &str) -> Result<User, FirebaseError> {
    match parse_optional_user(reply)? {
        Some(user) => Ok(user),
        None => Err(FirebaseError::Native {
            message: "expected signed-in user in host reply".into(),
        }),
    }
}

/// Parse `uid\x1femail\x1fdisplayName[\x1fproviderIds[\x1fphotoURL]]`.
///
/// `providerIds` is comma-separated (e.g. `password,google.com`). Missing 4th
/// field yields an empty `provider_ids` list. Missing 5th yields `photo_url = None`.
pub(crate) fn parse_user_fields(fields: &str) -> Result<User, FirebaseError> {
    let mut parts = fields.split(UNIT);
    let uid = parts.next().unwrap_or_default().to_string();
    let email = parts.next().unwrap_or_default().to_string();
    let display_name = parts.next().unwrap_or_default().to_string();
    let provider_ids = parts.next().unwrap_or_default().to_string();
    let photo_url = parts.next().unwrap_or_default().to_string();
    if uid.trim().is_empty() {
        return Err(FirebaseError::Native {
            message: "user uid missing in host reply".into(),
        });
    }
    Ok(User::from_host_fields(
        uid,
        email,
        display_name,
        provider_ids,
        photo_url,
    ))
}

fn parse_ok_body(reply: &str) -> Result<Option<&str>, FirebaseError> {
    let reply = reply.trim_end_matches('\0');
    if let Some(rest) = reply.strip_prefix("OK") {
        if rest.is_empty() {
            return Ok(None);
        }
        if let Some(body) = rest.strip_prefix(UNIT) {
            return Ok(Some(body));
        }
        return Err(FirebaseError::Native {
            message: format!("malformed OK reply: {reply}"),
        });
    }
    Err(parse_err(reply))
}

fn parse_err(reply: &str) -> FirebaseError {
    if let Some(rest) = reply.strip_prefix("ERR") {
        let rest = rest.strip_prefix(UNIT).unwrap_or(rest);
        let mut parts = rest.splitn(2, UNIT);
        let code = parts.next().unwrap_or("unknown").to_string();
        let message = parts.next().unwrap_or("").to_string();
        if code == "native" || code.is_empty() {
            return FirebaseError::Native {
                message: if message.is_empty() {
                    reply.to_string()
                } else {
                    message
                },
            };
        }
        return FirebaseError::Auth { code, message };
    }
    FirebaseError::Native {
        message: reply.to_string(),
    }
}

/// Shared email/password argument checks (desktop unit tests + native path).
pub(crate) fn require_email_password(email: &str, password: &str) -> Result<(), FirebaseError> {
    require_email(email)?;
    if password.is_empty() {
        return Err(FirebaseError::InvalidConfig(
            "password must not be empty".into(),
        ));
    }
    Ok(())
}

pub(crate) fn require_email(email: &str) -> Result<(), FirebaseError> {
    if email.trim().is_empty() {
        return Err(FirebaseError::InvalidConfig(
            "email must not be empty".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile_prefer::{
        prefer_profile_fields, MockProvider, MockTopLevel, PROVIDER_GOOGLE,
    };
    use crate::test_support::{err_reply, ok_user_reply, sample_user, user_wire};
    use rstest::rstest;

    #[rstest]
    #[case("OK", Ok(None))]
    #[case("OK\0", Ok(None))]
    #[case("OK\x1fuid1\x1fa@b.c\x1fAda", Ok(Some(sample_user("uid1", "a@b.c", "Ada", &[], ""))))]
    #[case("OK\x1fuid1\x1f\x1f", Ok(Some(sample_user("uid1", "", "", &[], ""))))]
    #[case(
        "OK\x1fuid1\x1fa@b.c\x1fAda\x1fpassword,google.com",
        Ok(Some(sample_user(
            "uid1",
            "a@b.c",
            "Ada",
            &["password", "google.com"],
            ""
        )))
    )]
    #[case(
        "OK\x1fuid1\x1f\x1f\x1fapple.com",
        Ok(Some(sample_user("uid1", "", "", &["apple.com"], "")))
    )]
    #[case(
        "OK\x1fuid1\x1fa@b.c\x1fAda\x1f",
        Ok(Some(sample_user("uid1", "a@b.c", "Ada", &[], "")))
    )]
    #[case(
        "OK\x1fuid1\x1fg@ex.com\x1fAda\x1fpassword,google.com\x1fhttps://g/p.png",
        Ok(Some(sample_user(
            "uid1",
            "g@ex.com",
            "Ada",
            &["password", "google.com"],
            "https://g/p.png"
        )))
    )]
    #[case(
        "OK\x1fuid1\x1fa@b.c\x1fAda\x1fpassword\x1f",
        Ok(Some(sample_user("uid1", "a@b.c", "Ada", &["password"], "")))
    )]
    fn test_parse_optional_user(
        #[case] input: &str,
        #[case] expected: Result<Option<User>, FirebaseError>,
    ) {
        assert_eq!(parse_optional_user(input), expected);
    }

    #[rstest]
    #[case(
        "OK\x1fuid1\x1fa@b.c\x1fAda\x1fpassword,apple.com",
        sample_user("uid1", "a@b.c", "Ada", &["password", "apple.com"], "")
    )]
    fn test_parse_user_with_providers(#[case] input: &str, #[case] expected: User) {
        assert_eq!(parse_user(input).unwrap(), expected);
    }

    #[rstest]
    #[case("OK\x1f\x1f\x1f")]
    #[case("OK\x1f   \x1fa@b.c\x1fAda")]
    fn test_parse_user_missing_uid(#[case] input: &str) {
        let err = parse_user(input).unwrap_err();
        assert!(matches!(err, FirebaseError::Native { .. }));
    }

    #[rstest]
    #[case("invalid-credential", "Wrong password")]
    #[case("email-already-in-use", "Taken")]
    #[case("native", "Crash message")]
    #[case("cancelled", "user cancelled")]
    #[case("unavailable", "google_web_client_id is required")]
    #[case("account-exists-with-different-credential", "Exists")]
    #[case("credential-already-in-use", "In use")]
    #[case("provider-already-linked", "Linked")]
    #[case("no-such-provider", "Missing")]
    #[case("requires-recent-login", "Sign in again")]
    fn test_parse_error_codes(#[case] expected_code: &str, #[case] expected_msg: &str) {
        let raw = err_reply(expected_code, expected_msg);
        let err = parse_void(&raw).unwrap_err();
        match err {
            FirebaseError::Auth { code, message } => {
                assert_eq!(code, expected_code);
                assert_eq!(message, expected_msg);
            }
            FirebaseError::Native { message } => {
                assert_eq!(expected_code, "native");
                assert_eq!(message, expected_msg);
            }
            _ => panic!("unexpected error variant: {err:?}"),
        }
    }

    #[test]
    fn test_prefer_then_wire_roundtrip_google_linked() {
        let top = MockTopLevel {
            email: "pw@ex.com",
            name: "App Name",
            photo: "",
        };
        let providers = [
            MockProvider {
                id: "password",
                email: "pw@ex.com",
                name: "Pw",
                photo: "",
            },
            MockProvider {
                id: PROVIDER_GOOGLE,
                email: "g@ex.com",
                name: "Google Name",
                photo: "https://g/p.png",
            },
        ];
        let (email, name, photo) = prefer_profile_fields(top, &providers);
        let wire = user_wire("uid1", &email, &name, "password,google.com", Some(&photo));
        let user = parse_user_fields(&wire).unwrap();
        assert_eq!(user.email.as_deref(), Some("g@ex.com"));
        assert_eq!(user.display_name.as_deref(), Some("App Name"));
        assert_eq!(user.photo_url.as_deref(), Some("https://g/p.png"));
        assert_eq!(
            parse_optional_user(&ok_user_reply(
                "uid1",
                &email,
                &name,
                "password,google.com",
                Some(&photo)
            ))
            .unwrap()
            .unwrap(),
            user
        );
    }

    #[test]
    fn test_prefer_then_wire_roundtrip_apple_only() {
        let top = MockTopLevel {
            email: "",
            name: "",
            photo: "",
        };
        let providers = [MockProvider {
            id: "apple.com",
            email: "a@privaterelay.appleid.com",
            name: "",
            photo: "",
        }];
        let (email, name, photo) = prefer_profile_fields(top, &providers);
        let wire = user_wire("uid2", &email, &name, "apple.com", Some(&photo));
        let user = parse_user_fields(&wire).unwrap();
        assert_eq!(user.email.as_deref(), Some("a@privaterelay.appleid.com"));
        assert_eq!(user.display_name, None);
        assert_eq!(user.photo_url, None);
    }
}
