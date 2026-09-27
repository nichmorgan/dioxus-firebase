//! iOS: call the ObjC-visible Swift host shim.
//!
//! The app must:
//! 1. Add FirebaseAuth via Swift Package Manager.
//! 2. Compile [`ios/DioxusFirebaseAuthHost.swift`](../../../ios/DioxusFirebaseAuthHost.swift)
//!    into the Dioxus iOS target.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use objc::runtime::{Class, Object};
use objc::{class, msg_send, sel, sel_impl};

use crate::config::FirebaseOptions;
use crate::error::FirebaseError;
use crate::host_protocol::{parse_optional_string, parse_optional_user, parse_user, parse_void};
use crate::subscribe;
use crate::user::User;

const HOST_CLASS_NAME: &str = "DioxusFirebaseAuthHost";

pub(crate) fn initialize(options: &FirebaseOptions) -> Result<(), FirebaseError> {
    let host = host_class()?;
    let api_key = nsstring(&options.api_key)?;
    let app_id = nsstring(&options.app_id)?;
    let project_id = nsstring(&options.project_id)?;
    let messaging_sender_id = nsstring(options.messaging_sender_id.as_deref().unwrap_or(""))?;
    let storage_bucket = nsstring(options.storage_bucket.as_deref().unwrap_or(""))?;
    let database_url = nsstring(options.database_url.as_deref().unwrap_or(""))?;

    let err: *mut Object = unsafe {
        msg_send![
            host,
            initializeWithApiKey: api_key
            appId: app_id
            projectId: project_id
            messagingSenderId: messaging_sender_id
            storageBucket: storage_bucket
            databaseURL: database_url
        ]
    };

    map_reply(err, parse_void)
}

pub(crate) fn sign_in_with_email(email: &str, password: &str) -> Result<User, FirebaseError> {
    let host = host_class()?;
    let email = nsstring(email)?;
    let password = nsstring(password)?;
    let reply: *mut Object = unsafe { msg_send![host, signInWithEmail: email password: password] };
    map_reply(reply, parse_user)
}

pub(crate) fn create_user_with_email(email: &str, password: &str) -> Result<User, FirebaseError> {
    let host = host_class()?;
    let email = nsstring(email)?;
    let password = nsstring(password)?;
    let reply: *mut Object =
        unsafe { msg_send![host, createUserWithEmail: email password: password] };
    map_reply(reply, parse_user)
}

pub(crate) fn update_display_name(display_name: &str) -> Result<(), FirebaseError> {
    let host = host_class()?;
    let name = nsstring(display_name)?;
    let reply: *mut Object = unsafe { msg_send![host, updateDisplayName: name] };
    map_reply(reply, parse_void)
}

pub(crate) fn send_password_reset_email(email: &str) -> Result<(), FirebaseError> {
    let host = host_class()?;
    let email = nsstring(email)?;
    let reply: *mut Object = unsafe { msg_send![host, sendPasswordResetEmail: email] };
    map_reply(reply, parse_void)
}

pub(crate) fn sign_out() -> Result<(), FirebaseError> {
    let host = host_class()?;
    let reply: *mut Object = unsafe { msg_send![host, signOut] };
    map_reply(reply, parse_void)
}

pub(crate) fn current_user() -> Result<Option<User>, FirebaseError> {
    let host = host_class()?;
    let reply: *mut Object = unsafe { msg_send![host, currentUser] };
    map_reply(reply, parse_optional_user)
}

pub(crate) fn id_token(force_refresh: bool) -> Result<Option<String>, FirebaseError> {
    let host = host_class()?;
    let reply: *mut Object =
        unsafe { msg_send![host, idTokenWithForceRefresh: force_refresh as u8] };
    map_reply(reply, parse_optional_string)
}

pub(crate) fn use_emulator(host_name: &str, port: u16) -> Result<(), FirebaseError> {
    let host = host_class()?;
    let host_str = nsstring(host_name)?;
    let reply: *mut Object =
        unsafe { msg_send![host, useEmulatorWithHost: host_str port: port as u32] };
    map_reply(reply, parse_void)
}

pub(crate) fn subscribe_auth_state() -> Result<(), FirebaseError> {
    let host = host_class()?;
    let callback = dioxus_firebase_auth_state_changed as *const std::os::raw::c_void;
    let reply: *mut Object = unsafe { msg_send![host, subscribeAuthStateWithCallback: callback] };
    map_reply(reply, parse_void)
}

#[no_mangle]
pub unsafe extern "C" fn dioxus_firebase_auth_state_changed(json: *const c_char) {
    let payload = if json.is_null() {
        None
    } else {
        CStr::from_ptr(json).to_str().ok().map(str::to_owned)
    };
    subscribe::dispatch_auth_state_payload(payload.as_deref());
}

fn host_class() -> Result<&'static Class, FirebaseError> {
    Class::get(HOST_CLASS_NAME).ok_or_else(|| {
        FirebaseError::HostMissing(
            format!(
                "{HOST_CLASS_NAME} not found. Compile ios/DioxusFirebaseAuthHost.swift into the iOS app and link FirebaseAuth."
            ),
        )
    })
}

fn map_reply<T>(
    reply: *mut Object,
    parse: fn(&str) -> Result<T, FirebaseError>,
) -> Result<T, FirebaseError> {
    let message = nsstring_to_rust(reply).ok_or_else(|| FirebaseError::Native {
        message: "host returned null".into(),
    })?;
    parse(&message)
}

fn nsstring(s: &str) -> Result<*mut Object, FirebaseError> {
    let c = CString::new(s).map_err(|_| FirebaseError::Native {
        message: "string contained interior NUL".into(),
    })?;
    let bytes = c.as_bytes();
    let ns_string: *mut Object = unsafe {
        let alloc: *mut Object = msg_send![class!(NSString), alloc];
        msg_send![
            alloc,
            initWithBytes: bytes.as_ptr()
            length: bytes.len()
            encoding: 4usize // NSUTF8StringEncoding
        ]
    };
    if ns_string.is_null() {
        Err(FirebaseError::Native {
            message: "failed to allocate NSString".into(),
        })
    } else {
        Ok(ns_string)
    }
}

fn nsstring_to_rust(s: *mut Object) -> Option<String> {
    if s.is_null() {
        return None;
    }
    let cstr: *const i8 = unsafe { msg_send![s, UTF8String] };
    if cstr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(cstr) }
        .to_str()
        .ok()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_host_class_name() {
        assert_eq!(HOST_CLASS_NAME, "DioxusFirebaseAuthHost");
    }

    #[rstest]
    fn test_host_class_returns_host_missing_without_swift_host() {
        let err = host_class().expect_err("must fail without Swift host");
        assert!(
            matches!(err, FirebaseError::HostMissing(ref msg) if msg.contains(HOST_CLASS_NAME)),
            "unexpected error: {err:?}"
        );
    }

    #[rstest]
    fn test_primitives_return_host_missing_when_host_absent() {
        let opts = FirebaseOptions {
            api_key: "key".into(),
            app_id: "app".into(),
            project_id: "project".into(),
            messaging_sender_id: None,
            storage_bucket: None,
            database_url: None,
        };

        assert!(matches!(
            initialize(&opts).unwrap_err(),
            FirebaseError::HostMissing(_)
        ));
        assert!(matches!(
            sign_in_with_email("a@b.c", "pass").unwrap_err(),
            FirebaseError::HostMissing(_)
        ));
        assert!(matches!(
            create_user_with_email("a@b.c", "pass").unwrap_err(),
            FirebaseError::HostMissing(_)
        ));
        assert!(matches!(
            update_display_name("Ada").unwrap_err(),
            FirebaseError::HostMissing(_)
        ));
        assert!(matches!(
            send_password_reset_email("a@b.c").unwrap_err(),
            FirebaseError::HostMissing(_)
        ));
        assert!(matches!(
            sign_out().unwrap_err(),
            FirebaseError::HostMissing(_)
        ));
        assert!(matches!(
            current_user().unwrap_err(),
            FirebaseError::HostMissing(_)
        ));
        assert!(matches!(
            id_token(false).unwrap_err(),
            FirebaseError::HostMissing(_)
        ));
        assert!(matches!(
            use_emulator("127.0.0.1", 9099).unwrap_err(),
            FirebaseError::HostMissing(_)
        ));
        assert!(matches!(
            subscribe_auth_state().unwrap_err(),
            FirebaseError::HostMissing(_)
        ));
    }

    #[rstest]
    fn test_nsstring_rejects_interior_nul() {
        let err = nsstring("a\0b").expect_err("interior NUL must fail");
        assert!(matches!(
            err,
            FirebaseError::Native { message } if message.contains("interior NUL")
        ));
    }
}
