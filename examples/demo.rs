//! Home screen of square feature buttons for Firebase Auth on mobile.
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

const STYLES: &str = r#"
body { font-family: sans-serif; margin: 0; }
h1 { font-size: 1.4rem; margin: 16px; }
h3 { margin: 0 0 12px; }
.grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 16px;
    padding: 16px;
}
.tile {
    aspect-ratio: 1;
    border: 0;
    border-radius: 16px;
    background: #3d3a8c;
    color: white;
    font-size: 1.05rem;
    font-weight: 600;
}
.back {
    margin: 16px 16px 0;
    padding: 8px 0;
    border: 0;
    background: transparent;
    font-size: 1rem;
}
.status {
    margin: 16px;
    background: #eef2f5;
    padding: 10px;
    border-radius: 6px;
    white-space: pre-wrap;
    word-break: break-word;
}
.panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 16px;
}
.row {
    display: flex;
    gap: 10px;
}
.row button { flex: 1; }
.btn {
    padding: 10px;
    border: none;
    border-radius: 4px;
    color: white;
    font-size: 1rem;
}
.btn-primary { background: #0066cc; }
.btn-success { background: #28a745; }
.btn-google { background: #db4437; }
.btn-apple { background: #000000; }
.btn-info { background: #17a2b8; }
.btn-muted { background: #6c757d; }
.btn-danger { background: #dc3545; margin-top: 10px; }
input {
    padding: 8px;
    font-size: 1rem;
}
"#;

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Home,
    Email,
    Google,
    // Apple Sign-In blocked for now — re-enable when Console + OAuth are ready.
    // Apple,
    Account,
}

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
    let mut screen = use_signal(|| Screen::Home);
    let mut auth_state = use_signal(|| AuthState::Uninitialized);
    let mut auth_subscription = use_signal(|| None::<AuthStateSubscription>);
    let mut status_message = use_signal(|| String::from("Initializing..."));
    let email = use_signal(|| String::from("user@example.com"));
    let password = use_signal(|| String::from("Secret123!"));
    let display_name = use_signal(|| String::from("Dioxus Mobile User"));

    use_effect(move || {
        let api_key = option_env!("FIREBASE_API_KEY").unwrap_or("demo-api-key");
        let app_id = option_env!("FIREBASE_APP_ID").unwrap_or(DEMO_APP_ID);
        let project_id = option_env!("FIREBASE_PROJECT_ID").unwrap_or("demo-project");
        let use_auth_emulator = option_env!("FIREBASE_USE_AUTH_EMULATOR")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(option_env!("FIREBASE_PROJECT_ID").is_none());

        let options = FirebaseOptions {
            api_key: api_key.into(),
            app_id: app_id.into(),
            project_id: project_id.into(),
            messaging_sender_id: None,
            storage_bucket: None,
            database_url: None,
            google_web_client_id: option_env!("FIREBASE_GOOGLE_WEB_CLIENT_ID")
                .filter(|id| !id.is_empty())
                .map(str::to_owned),
        };

        match initialize(options) {
            Ok(()) => {
                if use_auth_emulator {
                    let _ = use_emulator(auth_emulator_host(), 9099);
                }

                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Option<User>>();

                match subscribe_auth_state(move |user| {
                    let _ = tx.send(user);
                }) {
                    Ok(sub) => {
                        auth_subscription.set(Some(sub));
                        status_message.set("Firebase Auth initialized & subscribed.".into());

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

    let current = screen();

    rsx! {
        style { {STYLES} }

        div {
            id: "auth-status",
            class: "status",
            strong { "Status: " }
            "{status_message}"
        }

        match current {
            Screen::Home => rsx! {
                h1 { "Firebase Auth" }
                if matches!(auth_state(), AuthState::Uninitialized) {
                    p { style: "margin: 16px;", "Connecting to Firebase Native Bridge..." }
                }
                div { class: "grid",
                    button {
                        class: "tile",
                        onclick: move |_| screen.set(Screen::Email),
                        "Email & Password"
                    }
                    button {
                        class: "tile",
                        onclick: move |_| screen.set(Screen::Google),
                        "Google"
                    }
                    // Apple Sign-In blocked for now — re-enable when Console + OAuth are ready.
                    // button {
                    //     class: "tile",
                    //     onclick: move |_| screen.set(Screen::Apple),
                    //     "Apple"
                    // }
                    button {
                        class: "tile",
                        onclick: move |_| screen.set(Screen::Account),
                        "Account"
                    }
                }
            },
            Screen::Email => rsx! {
                button {
                    class: "back",
                    onclick: move |_| screen.set(Screen::Home),
                    "Back"
                }
                h1 { "Email & Password" }
                {email_body(email, password, status_message, screen)}
            },
            Screen::Google => rsx! {
                button {
                    class: "back",
                    onclick: move |_| screen.set(Screen::Home),
                    "Back"
                }
                h1 { "Google" }
                {google_body(auth_state, status_message, screen)}
            },
            // Apple Sign-In blocked for now — re-enable when Console + OAuth are ready.
            // Screen::Apple => rsx! {
            //     button {
            //         class: "back",
            //         onclick: move |_| screen.set(Screen::Home),
            //         "Back"
            //     }
            //     h1 { "Apple" }
            //     {apple_body(auth_state, status_message, screen)}
            // },
            Screen::Account => rsx! {
                button {
                    class: "back",
                    onclick: move |_| screen.set(Screen::Home),
                    "Back"
                }
                h1 { "User Account" }
                {account_body(auth_state, display_name, status_message)}
            },
        }
    }
}

fn email_body(
    mut email: Signal<String>,
    mut password: Signal<String>,
    mut status_message: Signal<String>,
    mut screen: Signal<Screen>,
) -> Element {
    rsx! {
        div { class: "panel",
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
            div { class: "row",
                button {
                    id: "sign-in",
                    class: "btn btn-primary",
                    onclick: move |_| {
                        let email = email.read().clone();
                        let password = password.read().clone();
                        spawn_auth(
                            move || sign_in_with_email(email, password),
                            move |result| match result {
                                Ok(user) => {
                                    status_message.set(format!("Signed in user: {}", user.uid));
                                    screen.set(Screen::Account);
                                }
                                Err(err) => status_message.set(format!("Sign in error: {err}")),
                            },
                        );
                    },
                    "Sign In"
                }
                button {
                    id: "sign-up",
                    class: "btn btn-success",
                    onclick: move |_| {
                        let email = email.read().clone();
                        let password = password.read().clone();
                        spawn_auth(
                            move || create_user_with_email(email, password),
                            move |result| match result {
                                Ok(user) => {
                                    status_message.set(format!("Created user: {}", user.uid));
                                    screen.set(Screen::Account);
                                }
                                Err(err) => status_message.set(format!("Sign up error: {err}")),
                            },
                        );
                    },
                    "Sign Up"
                }
            }
            button {
                class: "btn btn-muted",
                onclick: move |_| {
                    let email = email.read().clone();
                    spawn_auth(
                        move || send_password_reset_email(email),
                        move |result| match result {
                            Ok(()) => status_message.set("Password reset email sent.".into()),
                            Err(err) => {
                                status_message.set(format!("Password reset error: {err}"))
                            }
                        },
                    );
                },
                "Send Password Reset"
            }
        }
    }
}

fn google_body(
    auth_state: Signal<AuthState>,
    mut status_message: Signal<String>,
    mut screen: Signal<Screen>,
) -> Element {
    let signed_in = matches!(auth_state(), AuthState::SignedIn(_));
    rsx! {
        div { class: "panel",
            button {
                id: "sign-in-google",
                class: "btn btn-google",
                onclick: move |_| {
                    spawn_auth(
                        sign_in_with_google,
                        move |result| match result {
                            Ok(user) => {
                                status_message.set(format!(
                                    "Signed in with Google: {} ({})",
                                    user.uid,
                                    user.provider_ids.join(",")
                                ));
                                screen.set(Screen::Account);
                            }
                            Err(err) => {
                                status_message.set(format!("Google sign-in error: {err}"))
                            }
                        },
                    );
                },
                "Sign in with Google"
            }
            if signed_in {
                button {
                    id: "link-google",
                    class: "btn btn-google",
                    onclick: move |_| {
                        spawn_auth(
                            link_with_google,
                            move |result| match result {
                                Ok(user) => status_message.set(format!(
                                    "Linked Google: {}",
                                    user.provider_ids.join(",")
                                )),
                                Err(err) => {
                                    status_message.set(format!("Link Google error: {err}"))
                                }
                            },
                        );
                    },
                    "Link Google"
                }
            }
        }
    }
}

// Apple Sign-In blocked for now — re-enable when Console + OAuth are ready.
// fn apple_body(
//     auth_state: Signal<AuthState>,
//     mut status_message: Signal<String>,
//     mut screen: Signal<Screen>,
// ) -> Element {
//     let signed_in = matches!(auth_state(), AuthState::SignedIn(_));
//     rsx! {
//         div { class: "panel",
//             button {
//                 id: "sign-in-apple",
//                 class: "btn btn-apple",
//                 onclick: move |_| {
//                     spawn_auth(
//                         sign_in_with_apple,
//                         move |result| match result {
//                             Ok(user) => {
//                                 status_message.set(format!(
//                                     "Signed in with Apple: {} ({})",
//                                     user.uid,
//                                     user.provider_ids.join(",")
//                                 ));
//                                 screen.set(Screen::Account);
//                             }
//                             Err(err) => {
//                                 status_message.set(format!("Apple sign-in error: {err}"))
//                             }
//                         },
//                     );
//                 },
//                 "Sign in with Apple"
//             }
//             if signed_in {
//                 button {
//                     id: "link-apple",
//                     class: "btn btn-apple",
//                     onclick: move |_| {
//                         spawn_auth(
//                             link_with_apple,
//                             move |result| match result {
//                                 Ok(user) => status_message.set(format!(
//                                     "Linked Apple: {}",
//                                     user.provider_ids.join(",")
//                                 )),
//                                 Err(err) => {
//                                     status_message.set(format!("Link Apple error: {err}"))
//                                 }
//                             },
//                         );
//                     },
//                     "Link Apple"
//                 }
//             }
//         }
//     }
// }

fn account_body(
    auth_state: Signal<AuthState>,
    mut display_name: Signal<String>,
    mut status_message: Signal<String>,
) -> Element {
    match &*auth_state.read() {
        AuthState::Uninitialized => rsx! {
            p { style: "margin: 16px;", "Connecting to Firebase Native Bridge..." }
        },
        AuthState::SignedOut => rsx! {
            p { style: "margin: 16px;", "Sign in from Email or Google to manage your account." }
        },
        AuthState::SignedIn(user) => {
            let uid = user.uid.clone();
            let email = user.email.clone().unwrap_or_else(|| "N/A".into());
            let name = user.display_name.clone().unwrap_or_else(|| "N/A".into());
            let providers = user.provider_ids.join(", ");
            rsx! {
                div { class: "panel",
                    p {
                        strong { "UID: " }
                        "{uid}"
                    }
                    p {
                        strong { "Email: " }
                        "{email}"
                    }
                    p {
                        strong { "Display Name: " }
                        "{name}"
                    }
                    p {
                        strong { "Providers: " }
                        "{providers}"
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
                        class: "btn btn-info",
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

                    button {
                        class: "btn btn-muted",
                        onclick: move |_| {
                            spawn_auth(
                                move || id_token(true),
                                move |result| match result {
                                    Ok(Some(token)) => {
                                        status_message.set(format!(
                                            "ID Token (len {}): {}...",
                                            token.len(),
                                            &token[..token.len().min(15)],
                                        ))
                                    }
                                    Ok(None) => {
                                        status_message.set("No ID Token returned.".into())
                                    }
                                    Err(err) => {
                                        status_message.set(format!("Fetch token error: {err}"))
                                    }
                                },
                            );
                        },
                        "Fetch Fresh ID Token"
                    }

                    button {
                        id: "sign-out",
                        class: "btn btn-danger",
                        onclick: move |_| {
                            spawn_auth(sign_out, move |result| {
                                if let Err(err) = result {
                                    status_message.set(format!("Sign out error: {err}"));
                                }
                            });
                        },
                        "Sign Out"
                    }
                }
            }
        }
    }
}
