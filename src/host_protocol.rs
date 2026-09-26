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

/// Parse `uid\x1femail\x1fdisplayName` (auth-state payloads and OK user bodies).
pub(crate) fn parse_user_fields(fields: &str) -> Result<User, FirebaseError> {
    let mut parts = fields.split(UNIT);
    let uid = parts.next().unwrap_or_default().to_string();
    let email = parts.next().unwrap_or_default().to_string();
    let display_name = parts.next().unwrap_or_default().to_string();
    if uid.trim().is_empty() {
        return Err(FirebaseError::Native {
            message: "user uid missing in host reply".into(),
        });
    }
    Ok(User::from_host_fields(uid, email, display_name))
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
    use rstest::rstest;

    #[rstest]
    #[case("OK", Ok(None))]
    #[case("OK\0", Ok(None))]
    #[case("OK\x1fuid1\x1fa@b.c\x1fAda", Ok(Some(User {
        uid: "uid1".into(),
        email: Some("a@b.c".into()),
        display_name: Some("Ada".into()),
    })))]
    #[case("OK\x1fuid1\x1f\x1f", Ok(Some(User {
        uid: "uid1".into(),
        email: None,
        display_name: None,
    })))]
    fn test_parse_optional_user(
        #[case] input: &str,
        #[case] expected: Result<Option<User>, FirebaseError>,
    ) {
        assert_eq!(parse_optional_user(input), expected);
    }

    #[rstest]
    #[case(
        "ERR\x1finvalid-credential\x1fWrong password",
        "invalid-credential",
        "Wrong password"
    )]
    #[case(
        "ERR\x1femail-already-in-use\x1fTaken",
        "email-already-in-use",
        "Taken"
    )]
    #[case("ERR\x1fnative\x1fCrash message", "native", "Crash message")]
    fn test_parse_error_codes(
        #[case] raw: &str,
        #[case] expected_code: &str,
        #[case] expected_msg: &str,
    ) {
        let err = parse_void(raw).unwrap_err();
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
}
