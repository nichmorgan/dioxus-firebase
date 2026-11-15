//! Shared unit-test fixtures for wire / User construction.

#![cfg(test)]

use crate::host_protocol::UNIT;
use crate::user::User;

/// Build a [`User`] for assertions (empty strings → `None` fields).
pub(crate) fn sample_user(
    uid: &str,
    email: &str,
    display_name: &str,
    provider_ids: &[&str],
    photo_url: &str,
) -> User {
    User::from_host_fields(
        uid.into(),
        email.into(),
        display_name.into(),
        provider_ids.join(","),
        photo_url.into(),
    )
}

/// Auth-state / OK-user body: `uid\x1femail\x1fdisplayName\x1fproviderIds[\x1fphotoURL]`.
///
/// When `photo` is `None`, omits field 5 (4-field backcompat shape).
pub(crate) fn user_wire(
    uid: &str,
    email: &str,
    name: &str,
    providers: &str,
    photo: Option<&str>,
) -> String {
    let mut s = format!("{uid}{UNIT}{email}{UNIT}{name}{UNIT}{providers}");
    if let Some(photo) = photo {
        s.push(UNIT);
        s.push_str(photo);
    }
    s
}

pub(crate) fn ok_user_reply(
    uid: &str,
    email: &str,
    name: &str,
    providers: &str,
    photo: Option<&str>,
) -> String {
    format!("OK{UNIT}{}", user_wire(uid, email, name, providers, photo))
}

pub(crate) fn err_reply(code: &str, message: &str) -> String {
    format!("ERR{UNIT}{code}{UNIT}{message}")
}
