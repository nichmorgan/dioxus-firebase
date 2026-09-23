//! Android: call the Kotlin host via JNI.
//!
//! Bundled automatically by Dioxus CLI 0.7+ via manganis Android plugin metadata.
//! Context comes from `ndk_context` (initialized by Dioxus/wry).
//!
//! The host is an app class, so it is loaded with `Context.getClassLoader().loadClass`.
//! `Env::find_class` (JNI `FindClass`) from `attach_current_thread` only searches the
//! bootstrap loader and cannot see it. On release/arm64, jni 0.22 panics when that
//! failure sets `ExceptionCheck` but `exception_occurred()` returns `None`, before any
//! fallback can run. Framework classes may still use `find_class`.

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

pub(crate) fn initialize(options: &FirebaseOptions) -> Result<(), FirebaseError> {
    let (vm, context_raw) = java_vm_and_context()?;
    let api_key = options.api_key.clone();
    let app_id = options.app_id.clone();
    let project_id = options.project_id.clone();
    let messaging_sender_id = options.messaging_sender_id.clone().unwrap_or_default();
    let storage_bucket = options.storage_bucket.clone().unwrap_or_default();
    let database_url = options.database_url.clone().unwrap_or_default();

    vm.attach_current_thread(|env| {
        let context = unsafe { JObject::from_raw(env, context_raw) };
        let host = find_host_class(env, &context)?;
        let api_key = env.new_string(&api_key)?;
        let app_id = env.new_string(&app_id)?;
        let project_id = env.new_string(&project_id)?;
        let messaging_sender_id = env.new_string(&messaging_sender_id)?;
        let storage_bucket = env.new_string(&storage_bucket)?;
        let database_url = env.new_string(&database_url)?;

        let reply = env
            .call_static_method(
                &host,
                jni_str!("initialize"),
                jni_sig!(
                    "(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"
                ),
                &[
                    JValue::Object(&context),
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
    let (vm, context_raw) = java_vm_and_context()?;
    vm.attach_current_thread(|env| {
        let context = unsafe { JObject::from_raw(env, context_raw) };
        let host = find_host_class(env, &context)?;
        let reply = env
            .call_static_method(
                &host,
                jni_str!("idToken"),
                jni_sig!("(Z)Ljava/lang/String;"),
                &[JValue::Bool(force_refresh)],
            )
            .map_err(|e| map_exception(env, e, "DioxusFirebaseAuthHost.idToken"))?
            .l()?;
        map_string_reply(env, reply, parse_optional_string)
    })
}

pub(crate) fn use_emulator(host: &str, port: u16) -> Result<(), FirebaseError> {
    let (vm, context_raw) = java_vm_and_context()?;
    let host_name = host.to_string();
    vm.attach_current_thread(|env| {
        let context = unsafe { JObject::from_raw(env, context_raw) };
        let host_class = find_host_class(env, &context)?;
        let host_str = env.new_string(&host_name)?;
        let reply = env
            .call_static_method(
                &host_class,
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
    let (vm, context_raw) = java_vm_and_context()?;
    vm.attach_current_thread(|env| {
        let context = unsafe { JObject::from_raw(env, context_raw) };
        let host = find_host_class(env, &context)?;
        register_auth_callback(env, &host)?;
        let reply = env
            .call_static_method(
                &host,
                jni_str!("subscribeAuthState"),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )
            .map_err(|e| map_exception(env, e, "DioxusFirebaseAuthHost.subscribeAuthState"))?
            .l()?;
        map_string_reply(env, reply, parse_void)
    })
}

fn call_email_password<T>(
    method: &'static JNIStr,
    what: &str,
    email: &str,
    password: &str,
    parse: fn(&str) -> Result<T, FirebaseError>,
) -> Result<T, FirebaseError> {
    let (vm, context_raw) = java_vm_and_context()?;
    let email = email.to_string();
    let password = password.to_string();
    vm.attach_current_thread(|env| {
        let context = unsafe { JObject::from_raw(env, context_raw) };
        let host = find_host_class(env, &context)?;
        let email = env.new_string(&email)?;
        let password = env.new_string(&password)?;
        let reply = env
            .call_static_method(
                &host,
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
    let (vm, context_raw) = java_vm_and_context()?;
    let value = value.to_string();
    vm.attach_current_thread(|env| {
        let context = unsafe { JObject::from_raw(env, context_raw) };
        let host = find_host_class(env, &context)?;
        let value = env.new_string(&value)?;
        let reply = env
            .call_static_method(
                &host,
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
    let (vm, context_raw) = java_vm_and_context()?;
    vm.attach_current_thread(|env| {
        let context = unsafe { JObject::from_raw(env, context_raw) };
        let host = find_host_class(env, &context)?;
        let reply = env
            .call_static_method(&host, method, jni_sig!("()Ljava/lang/String;"), &[])
            .map_err(|e| map_exception(env, e, what))?
            .l()?;
        map_string_reply(env, reply, parse)
    })
}

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
    // ndk-context 0.1 has no try API; android_context() panics if wry/Dioxus
    // never called initialize_android_context (unit tests, too-early calls).
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
    let loader = env
        .call_method(
            context,
            jni_str!("getClassLoader"),
            jni_sig!("()Ljava/lang/ClassLoader;"),
            &[],
        )
        .map_err(|e| map_exception(env, e, "Context.getClassLoader"))?
        .l()
        .map_err(|e| FirebaseError::Native {
            message: format!("getClassLoader returned unexpected type: {e}"),
        })?;
    env.cast_local::<JClassLoader>(loader)
        .map_err(|e| FirebaseError::Native {
            message: format!("failed to cast ClassLoader: {e}"),
        })
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
