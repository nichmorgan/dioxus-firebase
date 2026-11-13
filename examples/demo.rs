//! Minimal Dioxus example showing Firebase Auth integration on mobile.
//!
//! Start the Auth emulator, then:
//! `dx serve --example demo --platform android`
//! `dx serve --example demo --platform ios`

#[cfg(target_os = "android")]
use std::ffi::CStr;

use dioxus::prelude::*;
use dioxus_firebase::prelude::*;

/// Firebase Auth stores the signed-in user in the keychain. A `dx` simulator
/// build is only ad-hoc signed, so it has no `application-identifier` or
/// `keychain-access-groups` and `SecItemAdd` fails with -34018. The simulator
/// reads those entitlements from this Mach-O section (Xcode's Simulated.xcent).
/// Device builds are codesigned by `dx --device` and must not embed this.
/// On Mach-O, `#[used]` is `no_dead_strip`, so the linker keeps this section.
#[cfg(all(target_os = "ios", target_abi = "sim"))]
#[used]
#[link_section = "__TEXT,__entitlements"]
static SIMULATOR_ENTITLEMENTS: [u8; include_bytes!("demo-simulator.entitlements").len()] =
    *include_bytes!("demo-simulator.entitlements");

#[derive(Clone, PartialEq)]
enum AuthState {
    Uninitialized,
    SignedOut,
    SignedIn(User),
}

/// Firebase iOS rejects an app id whose platform field is not `ios`.
const DEMO_APP_ID: &str = if cfg!(target_os = "ios") {
    "1:123456789:ios:0123456789abcdef"
} else {
    "1:123456789:android:demo"
};

/// Auth emulator host for this process.
///
/// The Android emulator aliases the development machine as `10.0.2.2`. A physical
/// Android device uses `127.0.0.1` after `adb reverse tcp:9099 tcp:9099`. The iOS
/// simulator shares the host loopback.
#[cfg(target_os = "android")]
fn auth_emulator_host() -> &'static str {
    let qemu = android_prop(c"ro.boot.qemu");
    let hardware = android_prop(c"ro.hardware");
    if qemu == "1" || hardware.contains("ranchu") || hardware.contains("goldfish") {
        "10.0.2.2"
    } else {
        "127.0.0.1"
    }
}

#[cfg(not(target_os = "android"))]
fn auth_emulator_host() -> &'static str {
    "127.0.0.1"
}

/// `PROP_VALUE_MAX` is 92. `__system_property_get` returns the length excluding the NUL.
#[cfg(target_os = "android")]
fn android_prop(name: &CStr) -> String {
    let mut value = [0u8; 92];
    let len = unsafe { libc::__system_property_get(name.as_ptr(), value.as_mut_ptr().cast()) };
    if len <= 0 {
        return String::new();
    }
    let len = (len as usize).min(value.len());
    String::from_utf8_lossy(&value[..len]).into_owned()
}

fn main() {
    launch(app);
}

/// Run a blocking Auth call off the UI thread, then apply the result on the Dioxus runtime.
fn spawn_auth<T, F, A>(work: F, apply: A)
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
    A: FnOnce(T) + 'static,
{
    spawn(async move {
        let (tx, rx) = tokio::sync::oneshot::channel();
        std::thread::spawn(move || {
            let _ = tx.send(work());
        });
        if let Ok(value) = rx.await {
            apply(value);
        }
    });
}

fn app() -> Element {
    // Dioxus reactive signals
    let mut auth_state = use_signal(|| AuthState::Uninitialized);
    let mut auth_subscription = use_signal(|| None::<AuthStateSubscription>);
    let mut status_message = use_signal(|| String::from("Initializing..."));
    let mut email = use_signal(|| String::from("user@example.com"));
    let mut password = use_signal(|| String::from("Secret123!"));
    let mut display_name = use_signal(|| String::from("Dioxus Mobile User"));

    // Initialize Firebase and subscribe to Auth state on first paint
    use_effect(move || {
        let options = FirebaseOptions {
            api_key: "demo-api-key".into(),
            app_id: DEMO_APP_ID.into(),
            project_id: "demo-project".into(),
            messaging_sender_id: None,
            storage_bucket: None,
            database_url: None,
            // Set a real OAuth web client ID to exercise Google sheets.
            google_web_client_id: None,
        };

        match initialize(options) {
            Ok(()) => {
                let _ = use_emulator(auth_emulator_host(), 9099);

                // Channel to bridge native background callback -> Dioxus UI signal
                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Option<User>>();

                // Subscribe to native Auth state changes (tx is Send + Sync)
                match subscribe_auth_state(move |user| {
                    let _ = tx.send(user);
                }) {
                    Ok(sub) => {
                        // Keep the guard alive; dropping it clears the listener.
                        auth_subscription.set(Some(sub));
                        status_message.set("Firebase Auth initialized & subscribed.".into());

                        // Fetch initial user session state immediately
                        match current_user() {
                            Ok(user) => {
                                auth_state.set(match user {
                                    Some(u) => AuthState::SignedIn(u),
                                    None => AuthState::SignedOut,
                                });
                            }
                            Err(_) => {
                                auth_state.set(AuthState::SignedOut);
                            }
                        }

                        // Spawn UI task to read future updates from channel
                        spawn(async move {
                            while let Some(user) = rx.recv().await {
                                auth_state.set(match user {
                                    Some(u) => AuthState::SignedIn(u),
                                    None => AuthState::SignedOut,
                                });
                            }
                        });
                    }
                    Err(err) => {
                        status_message.set(format!("Subscription failed: {err}"));
                    }
                }
            }
            Err(FirebaseError::UnsupportedPlatform) => {
                status_message.set("Running on Desktop (dioxus-firebase is mobile-only).".into());
                auth_state.set(AuthState::SignedOut);
            }
            Err(err) => {
                status_message.set(format!("Initialization failed: {err}"));
            }
        }
    });

    rsx! {
        div { style: "padding: 20px; font-family: sans-serif; max-width: 480px; margin: 0 auto;",

            h2 { "Dioxus Firebase Auth Demo" }

            div {
                id: "auth-status",
                style: "background: #eef2f5; padding: 10px; border-radius: 6px; margin-bottom: 20px;",
                strong { "Status: " }
                "{status_message}"
            }

            match &*auth_state.read() {
                AuthState::Uninitialized => rsx! {
                    p { "Connecting to Firebase Native Bridge..." }
                },
                AuthState::SignedOut => rsx! {
                    div { style: "display: flex; flex-direction: column; gap: 12px;",
                        h3 { "Sign In / Register" }

                        input {
                            r#type: "email",
                            placeholder: "Email address",
                            value: "{email}",
                            oninput: move |e| email.set(e.value()),
                        }
                        input {
                            r#type: "password",
                            placeholder: "Password",
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                        }

                        div { style: "display: flex; gap: 10px;",
                            button {
                                id: "sign-in",
                                style: "flex: 1; padding: 10px; background: #0066cc; color: white; border: none; border-radius: 4px;",
                                onclick: move |_| {
                                    let email = email.read().clone();
                                    let password = password.read().clone();
                                    spawn_auth(
                                        move || sign_in_with_email(email, password),
                                        move |result| match result {
                                            Ok(user) => status_message.set(format!("Signed in user: {}", user.uid)),
                                            Err(err) => status_message.set(format!("Sign in error: {err}")),
                                        },
                                    );
                                },
                                "Sign In"
                            }
                            button {
                                id: "sign-up",
                                style: "flex: 1; padding: 10px; background: #28a745; color: white; border: none; border-radius: 4px;",
                                onclick: move |_| {
                                    let email = email.read().clone();
                                    let password = password.read().clone();
                                    spawn_auth(
                                        move || create_user_with_email(email, password),
                                        move |result| match result {
                                            Ok(user) => status_message
                                                .set(format!("Created user: {}", user.uid)),
                                            Err(err) => status_message
                                                .set(format!("Sign up error: {err}")),
                                        },
                                    );
                                },
                                "Sign Up"
                            }
                        }

                        div { style: "display: flex; gap: 10px;",
                            button {
                                id: "sign-in-google",
                                style: "flex: 1; padding: 10px; background: #db4437; color: white; border: none; border-radius: 4px;",
                                onclick: move |_| {
                                    spawn_auth(
                                        sign_in_with_google,
                                        move |result| match result {
                                            Ok(user) => status_message.set(format!(
                                                "Signed in with Google: {} ({})",
                                                user.uid,
                                                user.provider_ids.join(",")
                                            )),
                                            Err(err) => status_message
                                                .set(format!("Google sign-in error: {err}")),
                                        },
                                    );
                                },
                                "Google"
                            }
                            button {
                                id: "sign-in-apple",
                                style: "flex: 1; padding: 10px; background: #000000; color: white; border: none; border-radius: 4px;",
                                onclick: move |_| {
                                    spawn_auth(
                                        sign_in_with_apple,
                                        move |result| match result {
                                            Ok(user) => status_message.set(format!(
                                                "Signed in with Apple: {} ({})",
                                                user.uid,
                                                user.provider_ids.join(",")
                                            )),
                                            Err(err) => status_message
                                                .set(format!("Apple sign-in error: {err}")),
                                        },
                                    );
                                },
                                "Apple"
                            }
                        }
                    }
                },
                AuthState::SignedIn(user) => rsx! {
                    div { style: "display: flex; flex-direction: column; gap: 10px;",
                        h3 { "User Account" }

                        p {
                            strong { "UID: " }
                            "{user.uid}"
                        }
                        p {
                            strong { "Email: " }
                            "{user.email.as_deref().unwrap_or(\"N/A\")}"
                        }
                        p {
                            strong { "Display Name: " }
                            "{user.display_name.as_deref().unwrap_or(\"N/A\")}"
                        }
                        p {
                            strong { "Providers: " }
                            "{user.provider_ids.join(\", \")}"
                        }

                        hr {}

                        label { "Update Profile Name:" }
                        input {
                            r#type: "text",
                            placeholder: "Display Name",
                            value: "{display_name}",
                            oninput: move |e| display_name.set(e.value()),
                        }

                        button {
                            id: "update-display-name",
                            style: "padding: 8px; background: #17a2b8; color: white; border: none; border-radius: 4px;",
                            onclick: move |_| {
                                let display_name = display_name.read().clone();
                                spawn_auth(
                                    move || update_display_name(display_name),
                                    move |result| match result {
                                        Ok(()) => status_message
                                            .set("Display name updated successfully!".into()),
                                        Err(err) => status_message
                                            .set(format!("Profile update failed: {err}")),
                                    },
                                );
                            },
                            "Update Display Name"
                        }

                        div { style: "display: flex; gap: 10px;",
                            button {
                                id: "link-google",
                                style: "flex: 1; padding: 8px; background: #db4437; color: white; border: none; border-radius: 4px;",
                                onclick: move |_| {
                                    spawn_auth(
                                        link_with_google,
                                        move |result| match result {
                                            Ok(user) => status_message.set(format!(
                                                "Linked Google: {}",
                                                user.provider_ids.join(",")
                                            )),
                                            Err(err) => status_message
                                                .set(format!("Link Google error: {err}")),
                                        },
                                    );
                                },
                                "Link Google"
                            }
                            button {
                                id: "link-apple",
                                style: "flex: 1; padding: 8px; background: #000000; color: white; border: none; border-radius: 4px;",
                                onclick: move |_| {
                                    spawn_auth(
                                        link_with_apple,
                                        move |result| match result {
                                            Ok(user) => status_message.set(format!(
                                                "Linked Apple: {}",
                                                user.provider_ids.join(",")
                                            )),
                                            Err(err) => status_message
                                                .set(format!("Link Apple error: {err}")),
                                        },
                                    );
                                },
                                "Link Apple"
                            }
                        }

                        button {
                            style: "padding: 8px; background: #6c757d; color: white; border: none; border-radius: 4px;",
                            onclick: move |_| {
                                spawn_auth(
                                    move || id_token(true),
                                    move |result| match result {
                                        Ok(Some(token)) => {
                                            status_message
                                                .set(
                                                    format!(
                                                        "ID Token (len {}): {}...",
                                                        token.len(),
                                                        &token[..token.len().min(15)],
                                                    ),
                                                )
                                        }
                                        Ok(None) => status_message.set("No ID Token returned.".into()),
                                        Err(err) => status_message.set(format!("Fetch token error: {err}")),
                                    },
                                );
                            },
                            "Fetch Fresh ID Token"
                        }

                        button {
                            id: "sign-out",
                            style: "padding: 8px; background: #dc3545; color: white; border: none; border-radius: 4px; margin-top: 10px;",
                            onclick: move |_| {
                                spawn_auth(
                                    sign_out,
                                    move |result| {
                                        if let Err(err) = result {
                                            status_message.set(format!("Sign out error: {err}"));
                                        }
                                    },
                                );
                            },
                            "Sign Out"
                        }
                    }
                },
            }
        }
    }
}
