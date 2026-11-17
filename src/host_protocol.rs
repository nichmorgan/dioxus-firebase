//! Shared host reply protocol (Kotlin / Swift ↔ Rust).
//!
//! Success: `OK` or `OK\x1f…`
//! Auth / native error: `ERR\x1f<code>\x1f<message>`

#![allow(dead_code)] // used from android/ios native modules

use crate::error::FirebaseError;
use crate::user::{nonempty, User, UserInfo};

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

/// Parse
/// `uid\x1femail\x1fdisplayName\x1fphotoURL\x1fN` then N×
/// `providerId\x1fuid\x1femail\x1fdisplayName\x1fphotoURL`.
///
/// Top-level fields are Firebase user values as-is (empty → `None`). Every
/// `providerData` row is included (including `firebase` when present).
pub(crate) fn parse_user_fields(fields: &str) -> Result<User, FirebaseError> {
    let mut parts = fields.split(UNIT);
    let uid = parts.next().unwrap_or_default().to_string();
    let email = parts.next().unwrap_or_default().to_string();
    let display_name = parts.next().unwrap_or_default().to_string();
    let photo_url = parts.next().unwrap_or_default().to_string();
    let n_raw = parts.next().unwrap_or("0");
    let n: usize = n_raw.parse().map_err(|_| FirebaseError::Native {
        message: format!("invalid provider count in host reply: {n_raw}"),
    })?;
    if uid.trim().is_empty() {
        return Err(FirebaseError::Native {
            message: "user uid missing in host reply".into(),
        });
    }

    let mut provider_data = Vec::with_capacity(n);
    for _ in 0..n {
        let provider_id = parts.next().unwrap_or_default().to_string();
        let p_uid = parts.next().unwrap_or_default().to_string();
        let p_email = parts.next().unwrap_or_default().to_string();
        let p_name = parts.next().unwrap_or_default().to_string();
        let p_photo = parts.next().unwrap_or_default().to_string();
        provider_data.push(UserInfo {
            provider_id,
            uid: nonempty(p_uid),
            email: nonempty(p_email),
            display_name: nonempty(p_name),
            photo_url: nonempty(p_photo),
        });
    }

    Ok(User::from_host_fields(
        uid,
        email,
        display_name,
        photo_url,
        provider_data,
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
        return FirebaseError::AuthException { code, message };
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
    use crate::test_support::{err_reply, ok_user_reply, sample_provider, sample_user, user_wire};
    use rstest::rstest;

    #[rstest]
    #[case("OK".to_string(), Ok(None))]
    #[case("OK\0".to_string(), Ok(None))]
    #[case(
        ok_user_reply("uid1", "a@b.c", "Ada", "", &[]),
        Ok(Some(sample_user("uid1", Some("a@b.c"), Some("Ada"), None, vec![])))
    )]
    #[case(
        ok_user_reply("uid1", "", "", "", &[]),
        Ok(Some(sample_user("uid1", None, None, None, vec![])))
    )]
    #[case(
        ok_user_reply(
            "uid1",
            "a@b.c",
            "Ada",
            "",
            &[("password", "uid1", "a@b.c", "Ada", "")],
        ),
        Ok(Some(sample_user(
            "uid1",
            Some("a@b.c"),
            Some("Ada"),
            None,
            vec![sample_provider(
                "password",
                Some("uid1"),
                Some("a@b.c"),
                Some("Ada"),
                None,
            )],
        )))
    )]
    #[case(
        ok_user_reply(
            "uid1",
            "pw@ex.com",
            "App",
            "",
            &[
                ("password", "uid1", "pw@ex.com", "Pw", ""),
                ("google.com", "g-uid", "g@ex.com", "Google", "https://g/p.png"),
            ],
        ),
        Ok(Some(sample_user(
            "uid1",
            Some("pw@ex.com"),
            Some("App"),
            None,
            vec![
                sample_provider(
                    "password",
                    Some("uid1"),
                    Some("pw@ex.com"),
                    Some("Pw"),
                    None,
                ),
                sample_provider(
                    "google.com",
                    Some("g-uid"),
                    Some("g@ex.com"),
                    Some("Google"),
                    Some("https://g/p.png"),
                ),
            ],
        )))
    )]
    #[case(
        ok_user_reply(
            "uid1",
            "a@b.c",
            "Ada",
            "https://ex/p.png",
            &[("password", "uid1", "a@b.c", "Ada", "")],
        ),
        Ok(Some(sample_user(
            "uid1",
            Some("a@b.c"),
            Some("Ada"),
            Some("https://ex/p.png"),
            vec![sample_provider(
                "password",
                Some("uid1"),
                Some("a@b.c"),
                Some("Ada"),
                None,
            )],
        )))
    )]
    fn test_parse_optional_user(
        #[case] input: String,
        #[case] expected: Result<Option<User>, FirebaseError>,
    ) {
        assert_eq!(parse_optional_user(&input), expected);
    }

    #[rstest]
    #[case(
        ok_user_reply(
            "uid1",
            "a@b.c",
            "Ada",
            "",
            &[
                ("password", "uid1", "a@b.c", "Ada", ""),
                ("apple.com", "a-uid", "a@privaterelay.appleid.com", "", ""),
            ],
        ),
        sample_user(
            "uid1",
            Some("a@b.c"),
            Some("Ada"),
            None,
            vec![
                sample_provider(
                    "password",
                    Some("uid1"),
                    Some("a@b.c"),
                    Some("Ada"),
                    None,
                ),
                sample_provider(
                    "apple.com",
                    Some("a-uid"),
                    Some("a@privaterelay.appleid.com"),
                    None,
                    None,
                ),
            ],
        )
    )]
    fn test_parse_user_with_providers(#[case] input: String, #[case] expected: User) {
        assert_eq!(parse_user(&input).unwrap(), expected);
    }

    #[rstest]
    #[case("OK\x1f\x1f\x1f\x1f\x1f0")]
    #[case("OK\x1f   \x1fa@b.c\x1fAda\x1f\x1f0")]
    fn test_parse_user_missing_uid(#[case] input: &str) {
        let err = parse_user(input).unwrap_err();
        assert!(matches!(err, FirebaseError::Native { .. }));
    }

    #[test]
    fn test_parse_user_invalid_provider_count() {
        let err = parse_user("OK\x1fuid1\x1fa@b.c\x1fAda\x1f\x1fxx").unwrap_err();
        assert!(matches!(err, FirebaseError::Native { .. }));
    }

    #[rstest]
    #[case("invalid-credential", "Wrong password")]
    #[case("email-already-in-use", "Taken")]
    #[case("native", "Crash message")]
    #[case("canceled", "user canceled")]
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
            FirebaseError::AuthException { code, message } => {
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
    fn test_wire_roundtrip_top_level_only() {
        let wire = user_wire("uid1", "a@b.c", "Ada", "", &[]);
        let user = parse_user_fields(&wire).unwrap();
        assert_eq!(user.email.as_deref(), Some("a@b.c"));
        assert_eq!(user.display_name.as_deref(), Some("Ada"));
        assert_eq!(user.photo_url, None);
        assert!(user.provider_data.is_empty());
        assert_eq!(
            parse_optional_user(&ok_user_reply("uid1", "a@b.c", "Ada", "", &[]))
                .unwrap()
                .unwrap(),
            user
        );
    }

    #[test]
    fn test_wire_roundtrip_password_and_google_distinct() {
        let providers = [
            ("password", "uid1", "pw@ex.com", "Pw", ""),
            (
                "google.com",
                "g-uid",
                "g@ex.com",
                "Google",
                "https://g/p.png",
            ),
        ];
        let wire = user_wire("uid1", "pw@ex.com", "App Name", "", &providers);
        let user = parse_user_fields(&wire).unwrap();
        // Top-level stays as hosts sent it — no Google/Apple preference.
        assert_eq!(user.email.as_deref(), Some("pw@ex.com"));
        assert_eq!(user.display_name.as_deref(), Some("App Name"));
        assert_eq!(user.photo_url, None);
        assert_eq!(user.provider_data.len(), 2);
        assert_eq!(user.provider_data[0].provider_id, "password");
        assert_eq!(user.provider_data[0].email.as_deref(), Some("pw@ex.com"));
        assert_eq!(user.provider_data[1].provider_id, "google.com");
        assert_eq!(user.provider_data[1].email.as_deref(), Some("g@ex.com"));
        assert_eq!(
            user.provider_data[1].photo_url.as_deref(),
            Some("https://g/p.png")
        );
    }

    #[test]
    fn test_wire_roundtrip_empty_photo_and_n_zero() {
        let wire = user_wire("uid2", "", "", "", &[]);
        let user = parse_user_fields(&wire).unwrap();
        assert_eq!(user.uid, "uid2");
        assert_eq!(user.email, None);
        assert_eq!(user.photo_url, None);
        assert_eq!(user.provider_data, vec![]);
    }

    #[test]
    fn test_wire_includes_firebase_provider_row() {
        let providers = [("firebase", "uid1", "a@b.c", "", "")];
        let user = parse_user_fields(&user_wire("uid1", "a@b.c", "", "", &providers)).unwrap();
        assert_eq!(user.provider_data.len(), 1);
        assert_eq!(user.provider_data[0].provider_id, "firebase");
    }
}
