//! iOS: call the ObjC-visible Swift host shim.
//!
//! Dioxus CLI 0.7+ embeds [`ios/`](../../../ios/) (SwiftPM product
//! `DioxusFirebaseAuthHost`, which depends on FirebaseAuth, FirebaseCore, and
//! GoogleSignIn). This module `dlopen`s that framework before looking up the
//! class. A non-`dx` build must add the local `ios/` package in Xcode instead.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::os::unix::ffi::OsStrExt;
use std::sync::OnceLock;

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
    let google_web_client_id = nsstring(options.google_web_client_id.as_deref().unwrap_or(""))?;

    let err: *mut Object = unsafe {
        msg_send![
            host,
            initializeWithApiKey: api_key
            appId: app_id
            projectId: project_id
            messagingSenderId: messaging_sender_id
            storageBucket: storage_bucket
            databaseURL: database_url
            googleWebClientId: google_web_client_id
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

pub(crate) fn sign_in_with_google() -> Result<User, FirebaseError> {
    let host = host_class()?;
    let reply: *mut Object = unsafe { msg_send![host, signInWithGoogle] };
    map_reply(reply, parse_user)
}

pub(crate) fn link_with_google() -> Result<User, FirebaseError> {
    let host = host_class()?;
    let reply: *mut Object = unsafe { msg_send![host, linkWithGoogle] };
    map_reply(reply, parse_user)
}

// Apple Sign-In blocked for now — re-enable when Console + OAuth are ready.
#[allow(dead_code)]
pub(crate) fn sign_in_with_apple() -> Result<User, FirebaseError> {
    let host = host_class()?;
    let reply: *mut Object = unsafe { msg_send![host, signInWithApple] };
    map_reply(reply, parse_user)
}

// Apple Sign-In blocked for now — re-enable when Console + OAuth are ready.
#[allow(dead_code)]
pub(crate) fn link_with_apple() -> Result<User, FirebaseError> {
    let host = host_class()?;
    let reply: *mut Object = unsafe { msg_send![host, linkWithApple] };
    map_reply(reply, parse_user)
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
    let load_err = load_bundled_swift_host();
    Class::get(HOST_CLASS_NAME).ok_or_else(|| {
        let detail = load_err.map(|err| format!(" ({err})")).unwrap_or_default();
        FirebaseError::HostMissing(format!(
            "{HOST_CLASS_NAME} not found{detail}. Build with Dioxus CLI 0.7+ (dx serve / dx bundle --platform ios) so this crate embeds ios/ (SwiftPM product {HOST_CLASS_NAME}). Without dx, add that local package in Xcode; it already depends on FirebaseAuth, FirebaseCore, and GoogleSignIn."
        ))
    })
}

/// `dx` compiles the Swift package after the Rust binary and installs it under
/// `Frameworks/`. The class is invisible to `Class::get` until that image is loaded.
///
/// Returns the first `dlopen` failure, if a candidate framework existed but did not load.
fn load_bundled_swift_host() -> Option<String> {
    #[link(name = "System")]
    extern "C" {
        fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    }

    const RTLD_NOW: c_int = 0x2;
    const RTLD_GLOBAL: c_int = 0x8;

    static LOADED: OnceLock<Option<String>> = OnceLock::new();
    LOADED
        .get_or_init(|| {
            let Ok(exe) = std::env::current_exe() else {
                return None;
            };
            let Some(dir) = exe.parent() else {
                return None;
            };
            let frameworks = dir.join("Frameworks");
            // Umbrella name when this plugin is first, then the product name when another
            // Swift plugin sorts ahead of `dioxusfirebaseauthhost`.
            let mut load_err = None;
            for name in ["DioxusSwiftPlugins", HOST_CLASS_NAME] {
                let path = frameworks.join(format!("{name}.framework/{name}"));
                if !path.is_file() {
                    continue;
                }
                let mut bytes = path.as_os_str().as_bytes().to_vec();
                if bytes.contains(&0) {
                    continue;
                }
                bytes.push(0);
                let Ok(cstr) = CString::from_vec_with_nul(bytes) else {
                    continue;
                };
                let handle = unsafe { dlopen(cstr.as_ptr(), RTLD_NOW | RTLD_GLOBAL) };
                if handle.is_null() {
                    if load_err.is_none() {
                        load_err = Some(dlerror_message());
                    }
                    continue;
                }
                if Class::get(HOST_CLASS_NAME).is_some() {
                    return None;
                }
            }
            load_err
        })
        .clone()
}

fn dlerror_message() -> String {
    #[link(name = "System")]
    extern "C" {
        fn dlerror() -> *const c_char;
    }

    let err = unsafe { dlerror() };
    if err.is_null() {
        return "dlopen failed".into();
    }
    unsafe { CStr::from_ptr(err) }
        .to_string_lossy()
        .into_owned()
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
            matches!(
                err,
                FirebaseError::HostMissing(ref msg)
                    if msg.contains(HOST_CLASS_NAME) && msg.contains("dx")
            ),
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
            google_web_client_id: None,
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
        assert!(matches!(
            sign_in_with_google().unwrap_err(),
            FirebaseError::HostMissing(_)
        ));
        // Apple Sign-In blocked for now — re-enable when Console + OAuth are ready.
        // assert!(matches!(
        //     sign_in_with_apple().unwrap_err(),
        //     FirebaseError::HostMissing(_)
        // ));
        assert!(matches!(
            link_with_google().unwrap_err(),
            FirebaseError::HostMissing(_)
        ));
        // assert!(matches!(
        //     link_with_apple().unwrap_err(),
        //     FirebaseError::HostMissing(_)
        // ));
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
