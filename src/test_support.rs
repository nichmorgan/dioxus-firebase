//! Shared unit-test fixtures for wire / User construction.

#![cfg(test)]

use crate::host_protocol::UNIT;
use crate::user::{User, UserInfo};

/// Build a [`UserInfo`] for assertions.
pub(crate) fn sample_provider(
    provider_id: &str,
    uid: Option<&str>,
    email: Option<&str>,
    display_name: Option<&str>,
    photo_url: Option<&str>,
) -> UserInfo {
    UserInfo {
        provider_id: provider_id.into(),
        uid: uid.map(str::to_owned),
        email: email.map(str::to_owned),
        display_name: display_name.map(str::to_owned),
        photo_url: photo_url.map(str::to_owned),
    }
}

/// Build a [`User`] for assertions.
pub(crate) fn sample_user(
    uid: &str,
    email: Option<&str>,
    display_name: Option<&str>,
    photo_url: Option<&str>,
    provider_data: Vec<UserInfo>,
) -> User {
    User {
        uid: uid.into(),
        email: email.map(str::to_owned),
        display_name: display_name.map(str::to_owned),
        photo_url: photo_url.map(str::to_owned),
        provider_data,
    }
}

/// Auth-state / OK-user body:
/// `uid\x1femail\x1fdisplayName\x1fphotoURL\x1fN` then N×
/// `providerId\x1fuid\x1femail\x1fdisplayName\x1fphotoURL`.
pub(crate) fn user_wire(
    uid: &str,
    email: &str,
    name: &str,
    photo: &str,
    providers: &[(
        &str, // providerId
        &str, // uid
        &str, // email
        &str, // displayName
        &str, // photoURL
    )],
) -> String {
    let mut s = format!(
        "{uid}{UNIT}{email}{UNIT}{name}{UNIT}{photo}{UNIT}{}",
        providers.len()
    );
    for (provider_id, p_uid, p_email, p_name, p_photo) in providers {
        s.push(UNIT);
        s.push_str(provider_id);
        s.push(UNIT);
        s.push_str(p_uid);
        s.push(UNIT);
        s.push_str(p_email);
        s.push(UNIT);
        s.push_str(p_name);
        s.push(UNIT);
        s.push_str(p_photo);
    }
    s
}

pub(crate) fn ok_user_reply(
    uid: &str,
    email: &str,
    name: &str,
    photo: &str,
    providers: &[(
        &str, // providerId
        &str, // uid
        &str, // email
        &str, // displayName
        &str, // photoURL
    )],
) -> String {
    format!("OK{UNIT}{}", user_wire(uid, email, name, photo, providers))
}

pub(crate) fn err_reply(code: &str, message: &str) -> String {
    format!("ERR{UNIT}{code}{UNIT}{message}")
}
