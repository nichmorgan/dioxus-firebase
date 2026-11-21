# dioxus-firebase

**Unofficial** [Dioxus](https://dioxuslabs.com/) bridge for [Firebase Auth](https://firebase.google.com/docs/auth) (email/password, Google, Apple).

**Mobile only for now** (iOS and Android). Web and desktop return `UnsupportedPlatform`.

This crate is **not affiliated** with Google / Firebase.

## FlutterFire

Inspired by [FlutterFire](https://github.com/firebase/flutterfire) `firebase_auth`. Platform paths follow those hosts. An extra exists only when an app cannot get the same result by calling the public API. Host names and platform notes: [docs/flutterfire.md](docs/flutterfire.md).

| Rust | Dart |
| -- | -- |
| `initialize_app` | `Firebase.initializeApp` |
| `sign_in_with_email_and_password` | `FirebaseAuth.signInWithEmailAndPassword` |
| `create_user_with_email_and_password` | `FirebaseAuth.createUserWithEmailAndPassword` |
| `update_display_name` | `User.updateDisplayName` |
| `update_photo_url` | `User.updatePhotoURL` |
| `send_password_reset_email` | `FirebaseAuth.sendPasswordResetEmail` |
| `sign_out` | `FirebaseAuth.signOut` |
| `current_user` | `FirebaseAuth.currentUser` |
| `get_id_token` | `User.getIdToken` |
| `use_auth_emulator` | `FirebaseAuth.useAuthEmulator` |
| `auth_state_changes` | `FirebaseAuth.authStateChanges` |
| `unlink` | `User.unlink` |
| `sign_in_with_google` | `FirebaseAuth.signInWithProvider(GoogleAuthProvider())` |
| `link_with_google` | `User.linkWithProvider(GoogleAuthProvider())` |
| `sign_in_with_apple` | `FirebaseAuth.signInWithProvider(AppleAuthProvider())` |
| `link_with_apple` | `User.linkWithProvider(AppleAuthProvider())` |
| `reload_current_user` | `User.reload` plus `User.getIdToken(true)` |
| `delete_current_user` | `User.reauthenticateWithProvider` plus `User.delete` |

Not wrapped: anonymous and custom-token sign-in, email-link sign-in, credential and standalone reauthenticate APIs, email verification, email or password update, a delete without reauth, action codes, phone auth, multi-factor, `idTokenChanges` and `userChanges`, other identity providers, and a standalone Apple authorization-code revoke.

Extras:

- `FirebaseOptions.google_web_client_id` — Google server client id, passed at init.
- `sign_in_with_google` and `link_with_google` — a fixed Google sheet instead of `signInWithProvider`.
- `sign_in_with_apple` and `link_with_apple` — a fixed Apple sheet. A second iOS call while one is up returns `operation-not-allowed`.
- Android copies a blank Apple `displayName` from the OAuth profile once. Apple sends that name only on the first authorization.
- `pick_google_credential` and `pick_apple_credential` — the same sheets, returning a `ProviderCredential` (ID token, plus the raw nonce for Apple) without signing in or linking. A backend links it with the Admin SDK. Apple pick is iOS only; Android returns `UnsupportedPlatform`.
- `delete_current_user("google.com" | "apple.com")` — Google deletes right away after a recent sign-in, and otherwise reauthenticates the signed-in Google account (auto-selected on Android, silent or hinted on iOS). Apple opens its sheet first and revokes its token (required by App Review on iOS). A dismiss changes nothing.
- `sign_out` also clears Android Credential Manager (best-effort) and iOS `GIDSignIn`.
- `is_initialized` — a local guard. FlutterFire does not expose one.

Link collisions return `credential-already-in-use` or `account-exists-with-different-credential`. The host does not sign in with the other account. Signed-out link, unlink, delete, and profile updates return `no-current-user`.

## Architecture

```text
Dioxus UI (Rust)
  → dioxus-firebase (Rust API)
    → Kotlin host (Android) / Swift host (iOS)
      → Firebase Auth SDK
```

Native calls go through a thin host — Kotlin on Android (auto-bundled by Dioxus CLI 0.7+), ObjC-visible Swift on iOS — not UniFFI.

## Ownership

| Layer | Owns |
| -- | -- |
| **Library** | Auth primitives, structured **wire** error codes, native persistence, token refresh, auth-state listener |
| **App** | `FirebaseOptions` (api key, app id, project id, optional Google web client ID, staging vs prod), Console provider enablement, Android SHA-1, iOS Google URL schemes, PT-BR / localized copy, route gates |

Do **not** put `google-services.json` keys inside this library. The app passes options at init.

## Quick start

Call `initialize_app` once before any other library API:

```rust
use dioxus_firebase::{initialize_app, FirebaseOptions};

initialize_app(FirebaseOptions {
    api_key: std::env::var("FIREBASE_API_KEY").expect("set FIREBASE_API_KEY"),
    app_id: std::env::var("FIREBASE_APP_ID").expect("set FIREBASE_APP_ID"),
    project_id: std::env::var("FIREBASE_PROJECT_ID").expect("set FIREBASE_PROJECT_ID"),
    messaging_sender_id: None,
    storage_bucket: None,
    database_url: None,
    google_web_client_id: std::env::var("FIREBASE_GOOGLE_WEB_CLIENT_ID").ok(),
})?;
```

Call mutations from `spawn_blocking` (or another worker thread). Social sheets present on the main thread while the worker waits. Full init, platform embedding, and iOS keychain notes: [docs/getting-started.md](docs/getting-started.md).

## Capabilities

- [Google sign-in / link](docs/google-sign-in.md) — Console setup, SHA-1, iOS URL schemes
- [Apple sign-in / link](docs/apple-sign-in.md) — Console grouping, Android OAuth, iOS capability
- [Auth-state + Dioxus signals](docs/auth-state.md) — prefer `auth_state_changes` over polling
- [Errors and linking](docs/errors-and-linking.md) — wire codes, link/unlink (no switch API)
- [Auth emulator](docs/auth-emulator.md) — `use_auth_emulator`, `10.0.2.2` vs `127.0.0.1`

## Try the demo

```bash
cp .env.example .env
make emulator
make android-demo   # or: make ios-demo
```

Emulator vs live `.env` modes, Maestro, and CI: [docs/demo-and-maestro.md](docs/demo-and-maestro.md).

## Documentation

| Guide | Topic |
| -- | -- |
| [Getting started](docs/getting-started.md) | Init, `spawn_blocking`, Android / iOS embedding |
| [Google sign-in](docs/google-sign-in.md) | Console, Credential Manager, iOS callbacks |
| [Apple sign-in](docs/apple-sign-in.md) | Console grouping, Android OAuth, iOS capability |
| [Auth-state](docs/auth-state.md) | Signals + channel pattern |
| [Errors and linking](docs/errors-and-linking.md) | Wire codes, link / unlink |
| [Auth emulator](docs/auth-emulator.md) | Hosts, `make emulator`, cleartext |
| [Demo and Maestro](docs/demo-and-maestro.md) | `.env`, `make *-demo`, UI suite |
| [Testing](docs/testing.md) | `cargo test`, `cross`, iOS sim |
| [Wire protocol](docs/wire-protocol.md) | `User` encoding (`\x1f`) |
| [FlutterFire](docs/flutterfire.md) | Host names, gaps, extras |

Index: [docs/README.md](docs/README.md).

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.
