//! Android: call the Kotlin host via JNI.
//!
//! Bundled automatically by Dioxus CLI 0.7+ via manganis Android plugin metadata.
//! Context comes from `ndk_context` (initialized by Dioxus/wry).
//!
//! The host is an app class, so it is loaded with `Context.getClassLoader().loadClass`.
//! `Env::find_class` (JNI `FindClass`) from `attach_current_thread` only searches the
//! bootstrap loader and cannot see app classes.

use jni::objects::{JClass, JClassLoader, JObject, JString, JValue};
use jni::strings::JNIStr;
use jni::sys::jobject;
use jni::{jni_sig, jni_str, native_method, Env, JavaVM, NativeMethod};

use crate::config::FirebaseOptions;
use crate::error::FirebaseError;
use crate::host_protocol::{parse_optional_string, parse_optional_user, parse_user, parse_void};
use crate::subscribe;
use crate::user::User;

const HOST_CLASS_DOT: &str = "io.dioxus.firebase.DioxusFirebaseAuthHost";

const ON_AUTH_STATE_CHANGED: NativeMethod = native_method! {
    java_type = "io.dioxus.firebase.DioxusFirebaseAuthHost",
    static fn on_auth_state_changed(user_json: JString) -> (),
};

fn on_auth_state_changed<'local>(
    env: &mut Env<'local>,
    _class: JClass<'local>,
    user_json: JString<'local>,
) -> Result<(), jni::errors::Error> {
    let payload = if user_json.is_null() {
        None
    } else {
        user_json.try_to_string(env).ok()
    };
    subscribe::dispatch_auth_state_payload(payload.as_deref());
    Ok(())
}

impl From<jni::errors::Error> for FirebaseError {
    fn from(error: jni::errors::Error) -> Self {
        Self::Native {
            message: error.to_string(),
        }
    }
}

// --- Public API ---

pub(crate) fn initialize(options: &FirebaseOptions) -> Result<(), FirebaseError> {
    let api_key = options.api_key.clone();
    let app_id = options.app_id.clone();
    let project_id = options.project_id.clone();
    let messaging_sender_id = options.messaging_sender_id.clone().unwrap_or_default();
    let storage_bucket = options.storage_bucket.clone().unwrap_or_default();
    let database_url = options.database_url.clone().unwrap_or_default();

    with_host(|env, host, context| {
        env.exception_clear();
        let api_key = env.new_string(&api_key)?;
        let app_id = env.new_string(&app_id)?;
        let project_id = env.new_string(&project_id)?;
        let messaging_sender_id = env.new_string(&messaging_sender_id)?;
        let storage_bucket = env.new_string(&storage_bucket)?;
        let database_url = env.new_string(&database_url)?;

        let reply = env
            .call_static_method(
                host,
                jni_str!("initialize"),
                jni_sig!(
                    "(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"
                ),
                &[
                    JValue::Object(context),
                    JValue::Object(&api_key),
                    JValue::Object(&app_id),
                    JValue::Object(&project_id),
                    JValue::Object(&messaging_sender_id),
                    JValue::Object(&storage_bucket),
                    JValue::Object(&database_url),
                ],
            )
            .map_err(|e| map_exception(env, e, "DioxusFirebaseAuthHost.initialize"))?
            .l()?;

        map_string_reply(env, reply, parse_void)
    })
}

pub(crate) fn sign_in_with_email(email: &str, password: &str) -> Result<User, FirebaseError> {
    call_email_password(
        jni_str!("signInWithEmail"),
        "DioxusFirebaseAuthHost.signInWithEmail",
        email,
        password,
        parse_user,
    )
}

pub(crate) fn create_user_with_email(email: &str, password: &str) -> Result<User, FirebaseError> {
    call_email_password(
        jni_str!("createUserWithEmail"),
        "DioxusFirebaseAuthHost.createUserWithEmail",
        email,
        password,
        parse_user,
    )
}

pub(crate) fn update_display_name(display_name: &str) -> Result<(), FirebaseError> {
    call_one_string(
        jni_str!("updateDisplayName"),
        "DioxusFirebaseAuthHost.updateDisplayName",
        display_name,
        parse_void,
    )
}

pub(crate) fn send_password_reset_email(email: &str) -> Result<(), FirebaseError> {
    call_one_string(
        jni_str!("sendPasswordResetEmail"),
        "DioxusFirebaseAuthHost.sendPasswordResetEmail",
        email,
        parse_void,
    )
}

pub(crate) fn sign_out() -> Result<(), FirebaseError> {
    call_no_args(
        jni_str!("signOut"),
        "DioxusFirebaseAuthHost.signOut",
        parse_void,
    )
}

pub(crate) fn current_user() -> Result<Option<User>, FirebaseError> {
    call_no_args(
        jni_str!("currentUser"),
        "DioxusFirebaseAuthHost.currentUser",
        parse_optional_user,
    )
}

pub(crate) fn id_token(force_refresh: bool) -> Result<Option<String>, FirebaseError> {
    with_host(|env, host, _| {
        let reply = env
            .call_static_method(
                host,
                jni_str!("idToken"),
                jni_sig!("(Z)Ljava/lang/String;"),
                &[JValue::Bool(force_refresh)],
            )
            .map_err(|e| map_exception(env, e, "DioxusFirebaseAuthHost.idToken"))?
            .l()?;
        map_string_reply(env, reply, parse_optional_string)
    })
}

pub(crate) fn use_emulator(host_name: &str, port: u16) -> Result<(), FirebaseError> {
    let host_name = host_name.to_string();
    with_host(|env, host, _| {
        let host_str = env.new_string(&host_name)?;
        let reply = env
            .call_static_method(
                host,
                jni_str!("useEmulator"),
                jni_sig!("(Ljava/lang/String;I)Ljava/lang/String;"),
                &[JValue::Object(&host_str), JValue::Int(i32::from(port))],
            )
            .map_err(|e| map_exception(env, e, "DioxusFirebaseAuthHost.useEmulator"))?
            .l()?;
        map_string_reply(env, reply, parse_void)
    })
}

pub(crate) fn subscribe_auth_state() -> Result<(), FirebaseError> {
    with_host(|env, host, _| {
        register_auth_callback(env, host)?;
        let reply = env
            .call_static_method(
                host,
                jni_str!("subscribeAuthState"),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )
            .map_err(|e| map_exception(env, e, "DioxusFirebaseAuthHost.subscribeAuthState"))?
            .l()?;
        map_string_reply(env, reply, parse_void)
    })
}

// --- JNI Dispatch Helpers ---

fn with_host<F, T>(f: F) -> Result<T, FirebaseError>
where
    F: FnOnce(&mut Env<'_>, &JClass<'_>, &JObject<'_>) -> Result<T, FirebaseError>,
{
    let (vm, context_raw) = java_vm_and_context()?;
    vm.attach_current_thread(|env| {
        let context = unsafe { JObject::from_raw(env, context_raw) };
        let host = find_host_class(env, &context)?;
        f(env, &host, &context)
    })
}

fn call_email_password<T>(
    method: &'static JNIStr,
    what: &str,
    email: &str,
    password: &str,
    parse: fn(&str) -> Result<T, FirebaseError>,
) -> Result<T, FirebaseError> {
    let email = email.to_string();
    let password = password.to_string();
    with_host(|env, host, _| {
        let email = env.new_string(&email)?;
        let password = env.new_string(&password)?;
        let reply = env
            .call_static_method(
                host,
                method,
                jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"),
                &[JValue::Object(&email), JValue::Object(&password)],
            )
            .map_err(|e| map_exception(env, e, what))?
            .l()?;
        map_string_reply(env, reply, parse)
    })
}

fn call_one_string<T>(
    method: &'static JNIStr,
    what: &str,
    value: &str,
    parse: fn(&str) -> Result<T, FirebaseError>,
) -> Result<T, FirebaseError> {
    let value = value.to_string();
    with_host(|env, host, _| {
        let value = env.new_string(&value)?;
        let reply = env
            .call_static_method(
                host,
                method,
                jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                &[JValue::Object(&value)],
            )
            .map_err(|e| map_exception(env, e, what))?
            .l()?;
        map_string_reply(env, reply, parse)
    })
}

fn call_no_args<T>(
    method: &'static JNIStr,
    what: &str,
    parse: fn(&str) -> Result<T, FirebaseError>,
) -> Result<T, FirebaseError> {
    with_host(|env, host, _| {
        let reply = env
            .call_static_method(host, method, jni_sig!("()Ljava/lang/String;"), &[])
            .map_err(|e| map_exception(env, e, what))?
            .l()?;
        map_string_reply(env, reply, parse)
    })
}

// --- ClassLoader & Low-Level JNI Resolution ---

fn register_auth_callback(env: &mut Env<'_>, host: &JClass<'_>) -> Result<(), FirebaseError> {
    unsafe { env.register_native_methods(host, &[ON_AUTH_STATE_CHANGED]) }.map_err(|e| {
        FirebaseError::Native {
            message: format!("failed to register onAuthStateChanged: {e}"),
        }
    })
}

fn java_vm_and_context() -> Result<(JavaVM, jobject), FirebaseError> {
    let missing = || {
        FirebaseError::HostMissing(
            "ndk_context is not initialized (is this a Dioxus Android app?)".into(),
        )
    };
    let android_ctx =
        std::panic::catch_unwind(ndk_context::android_context).map_err(|_| missing())?;
    if android_ctx.context().is_null() || android_ctx.vm().is_null() {
        return Err(missing());
    }
    let vm = unsafe { JavaVM::from_raw(android_ctx.vm().cast()) };
    Ok((vm, android_ctx.context() as jobject))
}

fn find_host_class<'a>(
    env: &mut Env<'a>,
    context: &JObject<'_>,
) -> Result<JClass<'a>, FirebaseError> {
    let loader = activity_class_loader(env, context)?;
    let class_name = env
        .new_string(HOST_CLASS_DOT)
        .map_err(|e| FirebaseError::Native {
            message: format!("failed to create class name string: {e}"),
        })?;
    let class = env
        .call_method(
            &loader,
            jni_str!("loadClass"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"),
            &[JValue::Object(&class_name)],
        )
        .map_err(|e| {
            env.exception_clear();
            FirebaseError::HostMissing(format!(
                "DioxusFirebaseAuthHost not found ({e}). Compile android/ DioxusFirebaseAuthHost.kt into the Android app and depend on Firebase Auth."
            ))
        })?
        .l()
        .map_err(|e| FirebaseError::Native {
            message: format!("loadClass returned unexpected type: {e}"),
        })?;
    env.cast_local::<JClass>(class)
        .map_err(|e| FirebaseError::Native {
            message: format!("failed to cast loaded host class: {e}"),
        })
}

fn activity_class_loader<'a>(
    env: &mut Env<'a>,
    context: &JObject<'_>,
) -> Result<JClassLoader<'a>, FirebaseError> {
    env.exception_clear();
    let loader_raw = raw_get_class_loader(env, context.as_raw())?;
    env.exception_clear();
    let loader = unsafe { JObject::from_raw(env, loader_raw) };
    env.cast_local::<JClassLoader>(loader)
        .map_err(|e| FirebaseError::Native {
            message: format!("failed to cast ClassLoader: {e}"),
        })
}

fn raw_get_class_loader(env: &mut Env<'_>, context: jobject) -> Result<jobject, FirebaseError> {
    unsafe {
        let jni_env = env.get_raw();
        let interface = *jni_env;
        ((*interface).v1_1.ExceptionClear)(jni_env);
        let class = ((*interface).v1_1.GetObjectClass)(jni_env, context);
        if class.is_null() {
            return Err(FirebaseError::Native {
                message: format!("Context.getClass: {}", pending_exception_text(env)),
            });
        }
        ((*interface).v1_1.ExceptionClear)(jni_env);
        let method = ((*interface).v1_1.GetMethodID)(
            jni_env,
            class,
            b"getClassLoader\0".as_ptr().cast(),
            b"()Ljava/lang/ClassLoader;\0".as_ptr().cast(),
        );
        if method.is_null() {
            return Err(FirebaseError::Native {
                message: format!("Context.getClassLoader: {}", pending_exception_text(env)),
            });
        }
        ((*interface).v1_1.ExceptionClear)(jni_env);
        let loader =
            ((*interface).v1_1.CallObjectMethodA)(jni_env, context, method, std::ptr::null());
        if loader.is_null() {
            return Err(FirebaseError::Native {
                message: format!("Context.getClassLoader: {}", pending_exception_text(env)),
            });
        }
        ((*interface).v1_1.ExceptionClear)(jni_env);
        Ok(loader)
    }
}

fn pending_exception_text(env: &mut Env<'_>) -> String {
    unsafe {
        let jni_env = env.get_raw();
        let interface = *jni_env;
        let throwable = ((*interface).v1_1.ExceptionOccurred)(jni_env);
        ((*interface).v1_1.ExceptionDescribe)(jni_env);
        ((*interface).v1_1.ExceptionClear)(jni_env);
        if throwable.is_null() {
            return "Java exception was thrown (ExceptionOccurred returned null)".into();
        }
        let class = ((*interface).v1_1.GetObjectClass)(jni_env, throwable);
        if class.is_null() {
            ((*interface).v1_1.ExceptionClear)(jni_env);
            return "Java exception was thrown (no throwable class)".into();
        }
        let to_string = ((*interface).v1_1.GetMethodID)(
            jni_env,
            class,
            b"toString\0".as_ptr().cast(),
            b"()Ljava/lang/String;\0".as_ptr().cast(),
        );
        if to_string.is_null() {
            ((*interface).v1_1.ExceptionClear)(jni_env);
            return "Java exception was thrown (toString missing)".into();
        }
        let message =
            ((*interface).v1_1.CallObjectMethodA)(jni_env, throwable, to_string, std::ptr::null());
        if message.is_null() || env.exception_check() {
            ((*interface).v1_1.ExceptionClear)(jni_env);
            return "Java exception was thrown (toString failed)".into();
        }
        let chars = ((*interface).v1_1.GetStringUTFChars)(
            jni_env,
            message as jni::sys::jstring,
            std::ptr::null_mut(),
        );
        if chars.is_null() {
            return "Java exception was thrown (empty message)".into();
        }
        let text = std::ffi::CStr::from_ptr(chars)
            .to_string_lossy()
            .into_owned();
        ((*interface).v1_1.ReleaseStringUTFChars)(jni_env, message as jni::sys::jstring, chars);
        text
    }
}

fn map_string_reply<T>(
    env: &mut Env<'_>,
    reply: JObject<'_>,
    parse: fn(&str) -> Result<T, FirebaseError>,
) -> Result<T, FirebaseError> {
    if reply.is_null() {
        return Err(FirebaseError::Native {
            message: "host returned null".into(),
        });
    }
    let jstring = env
        .cast_local::<JString>(reply)
        .map_err(|e| FirebaseError::Native {
            message: format!("host reply was not a String: {e}"),
        })?;
    let message = jstring
        .try_to_string(env)
        .map_err(|e| FirebaseError::Native {
            message: format!("failed to read host reply: {e}"),
        })?;
    parse(&message)
}

fn map_exception(env: &mut Env<'_>, err: jni::errors::Error, what: &str) -> FirebaseError {
    if env.exception_check() {
        let message = describe_exception(env).unwrap_or_else(|| err.to_string());
        env.exception_clear();
        return FirebaseError::Native {
            message: format!("{what}: {message}"),
        };
    }
    FirebaseError::Native {
        message: format!("{what}: {err}"),
    }
}

fn describe_exception(env: &mut Env<'_>) -> Option<String> {
    let throwable = env.exception_occurred()?;
    env.exception_clear();
    let message = env
        .call_method(
            &throwable,
            jni_str!("toString"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )
        .ok()?
        .l()
        .ok()?;
    let jstring = env.cast_local::<JString>(message).ok()?;
    jstring.try_to_string(env).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_host_class_dot_name() {
        assert_eq!(HOST_CLASS_DOT, "io.dioxus.firebase.DioxusFirebaseAuthHost");
    }

    #[rstest]
    fn test_java_vm_and_context_returns_host_missing_without_ndk_context() {
        let err = java_vm_and_context().expect_err("must fail without ndk_context");
        assert!(matches!(err, FirebaseError::HostMissing(msg) if msg.contains("ndk_context")));
    }

    #[rstest]
    fn test_primitives_return_host_missing_when_uncontextualized() {
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
}
