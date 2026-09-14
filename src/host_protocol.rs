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

    #[test]
    fn parses_signed_in_user() {
        let user = parse_optional_user("OK\u{1f}uid1\u{1f}a@b.c\u{1f}Ada")
            .unwrap()
            .unwrap();
        assert_eq!(user.uid, "uid1");
        assert_eq!(user.email.as_deref(), Some("a@b.c"));
        assert_eq!(user.display_name.as_deref(), Some("Ada"));
    }

    #[test]
    fn parses_signed_out() {
        assert!(parse_optional_user("OK").unwrap().is_none());
    }

    #[test]
    fn parses_optional_string() {
        assert_eq!(
            parse_optional_string("OK\u{1f}tok").unwrap().as_deref(),
            Some("tok")
        );
        assert!(parse_optional_string("OK").unwrap().is_none());
        assert!(parse_optional_string("OK\u{1f}").unwrap().is_none());
    }

    #[test]
    fn parses_auth_wire_code() {
        let err = parse_void("ERR\u{1f}email-already-in-use\u{1f}taken").unwrap_err();
        assert_eq!(
            err,
            FirebaseError::Auth {
                code: "email-already-in-use".into(),
                message: "taken".into(),
            }
        );
    }

    #[test]
    fn parses_native_timeout() {
        let err = parse_void("ERR\u{1f}native\u{1f}timeout").unwrap_err();
        assert_eq!(
            err,
            FirebaseError::Native {
                message: "timeout".into(),
            }
        );
    }

    #[test]
    fn rejects_malformed_ok() {
        let err = parse_optional_user("OKuid").unwrap_err();
        assert!(matches!(err, FirebaseError::Native { .. }));
    }

    #[test]
    fn parse_user_fields_rejects_blank_uid() {
        assert!(parse_user_fields("  \u{1f}a@b.c\u{1f}Ada").is_err());
    }
}
