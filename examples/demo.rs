//! Minimal Dioxus example showing Firebase Auth integration on mobile.
//!
//! Start the Auth emulator, then:
//! `dx serve --example demo --platform android`

use dioxus::prelude::*;
use dioxus_firebase::prelude::*;

#[derive(Clone, PartialEq)]
enum AuthState {
    Uninitialized,
    SignedOut,
    SignedIn(User),
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
            app_id: "1:123456789:android:demo".into(),
            project_id: "demo-project".into(),
            messaging_sender_id: None,
            storage_bucket: None,
            database_url: None,
        };

        match initialize(options) {
            Ok(()) => {
                // Connect to local Firebase Auth Emulator if running
                let _ = use_emulator("10.0.2.2", 9099);

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
        div {
            style: "padding: 20px; font-family: sans-serif; max-width: 480px; margin: 0 auto;",

            h2 { "Dioxus Firebase Auth Demo" }

            div {
                style: "background: #eef2f5; padding: 10px; border-radius: 6px; margin-bottom: 20px;",
                strong { "Status: " }
                "{status_message}"
            }

            match &*auth_state.read() {
                AuthState::Uninitialized => rsx! {
                    p { "Connecting to Firebase Native Bridge..." }
                },
                AuthState::SignedOut => rsx! {
                    div {
                        style: "display: flex; flex-direction: column; gap: 12px;",
                        h3 { "Sign In / Register" }

                        input {
                            r#type: "email",
                            placeholder: "Email address",
                            value: "{email}",
                            oninput: move |e| email.set(e.value())
                        }
                        input {
                            r#type: "password",
                            placeholder: "Password",
                            value: "{password}",
                            oninput: move |e| password.set(e.value())
                        }

                        div {
                            style: "display: flex; gap: 10px;",
                            button {
                                style: "flex: 1; padding: 10px; background: #0066cc; color: white; border: none; border-radius: 4px;",
                                onclick: move |_| {
                                    let email = email.read().clone();
                                    let password = password.read().clone();
                                    spawn_auth(
                                        move || sign_in_with_email(email, password),
                                        move |result| match result {
                                            Ok(user) => status_message.set(format!(
                                                "Signed in user: {}",
                                                user.uid
                                            )),
                                            Err(err) => status_message
                                                .set(format!("Sign in error: {err}")),
                                        },
                                    );
                                },
                                "Sign In"
                            }
                            button {
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
                    }
                },
                AuthState::SignedIn(user) => rsx! {
                    div {
                        style: "display: flex; flex-direction: column; gap: 10px;",
                        h3 { "User Account" }

                        p { strong { "UID: " } "{user.uid}" }
                        p { strong { "Email: " } "{user.email.as_deref().unwrap_or(\"N/A\")}" }
                        p { strong { "Display Name: " } "{user.display_name.as_deref().unwrap_or(\"N/A\")}" }

                        hr {}

                        label { "Update Profile Name:" }
                        input {
                            r#type: "text",
                            placeholder: "Display Name",
                            value: "{display_name}",
                            oninput: move |e| display_name.set(e.value())
                        }

                        button {
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

                        button {
                            style: "padding: 8px; background: #6c757d; color: white; border: none; border-radius: 4px;",
                            onclick: move |_| {
                                spawn_auth(move || id_token(true), move |result| match result {
                                    Ok(Some(token)) => status_message.set(format!(
                                        "ID Token (len {}): {}...",
                                        token.len(),
                                        &token[..token.len().min(15)]
                                    )),
                                    Ok(None) => {
                                        status_message.set("No ID Token returned.".into())
                                    }
                                    Err(err) => {
                                        status_message.set(format!("Fetch token error: {err}"))
                                    }
                                });
                            },
                            "Fetch Fresh ID Token"
                        }

                        button {
                            style: "padding: 8px; background: #dc3545; color: white; border: none; border-radius: 4px; margin-top: 10px;",
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
}
