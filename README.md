# dioxus-firebase

**Unofficial** [Dioxus](https://dioxuslabs.com/) bridge for [Firebase Auth](https://firebase.google.com/docs/auth) (email/password).

**Mobile only for now** (iOS and Android). Web and desktop return `UnsupportedPlatform`.

This crate is **not affiliated** with Google / Firebase. [JusFine](https://github.com/nichmorgan/jusfine-ui-dioxus) dogfoods it for production auth.

## Status

Early — v1 covers email/password only (no Google/Apple/phone, no email verification, no custom claims).

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
| **App** | `FirebaseOptions` (api key, app id, project id, staging vs prod), PT-BR / localized copy, route gates, fail-open identify |

Do **not** put `google-services.json` keys inside this library. The app passes options at init.

## Initialize

Call once before any other library API:

```rust
use dioxus_firebase::{initialize, FirebaseOptions};

initialize(FirebaseOptions {
    api_key: std::env::var("FIREBASE_API_KEY").expect("set FIREBASE_API_KEY"),
    app_id: std::env::var("FIREBASE_APP_ID").expect("set FIREBASE_APP_ID"),
    project_id: std::env::var("FIREBASE_PROJECT_ID").expect("set FIREBASE_PROJECT_ID"),
    messaging_sender_id: None,
    storage_bucket: None,
    database_url: None,
})?;
```

Double-init returns `AlreadyInitialized`. Desktop / web builds return `UnsupportedPlatform`.

Call mutations (`sign_in_with_email`, …) from `spawn_blocking` (or another worker thread) — not Dioxus `spawn`, which is often still the UI thread on iOS. The host hops to the main thread and **waits** on the Firebase Task.

## Auth-state + Dioxus signals

Prefer [`subscribe_auth_state`](fn@subscribe_auth_state) over polling. The native SDK restores the session; the callback fires for restore / sign-in / sign-out.

The callback may run off the UI thread. Pipe events into a channel and update a Signal from `spawn`:

```rust,ignore
use dioxus::prelude::*;
use dioxus_firebase::{subscribe_auth_state, User};

enum AuthStatus { Restoring, Guest, Ready(User) }

let mut status = use_signal(|| AuthStatus::Restoring);
let (tx, mut rx) = futures_channel::mpsc::unbounded();
let _sub = subscribe_auth_state(move |user| {
    let _ = tx.unbounded_send(user);
})?;
spawn(async move {
    use futures_util::StreamExt;
    while let Some(user) = rx.next().await {
        status.set(match user {
            Some(u) => AuthStatus::Ready(u),
            None => AuthStatus::Guest,
        });
    }
});
```

Keep the returned `AuthStateSubscription` alive (context / `use_hook`).

## Error codes

Failures return `FirebaseError::Auth { code, message }` with Firebase-style wire codes such as:

- `invalid-credential`
- `email-already-in-use`
- `invalid-email`
- `weak-password`
- `user-disabled`
- `too-many-requests`
- `network-request-failed`

Map codes to user-facing strings in the app (anti-enumeration on login, etc.).

## Emulator

Optional: `use_emulator(host, port)` after init. JusFine can keep using staging cloud in debug and adopt the emulator later.

On the Android emulator, call `use_emulator("10.0.2.2", 9099)`. `10.0.2.2` is the emulator's alias for the host machine, so no `adb reverse` is required. Start the Auth emulator on the host (`0.0.0.0:9099` in [`firebase.json`](firebase.json)):

```bash
npx -y firebase-tools emulators:start --only auth
```

Dioxus CLI's generated network-security config only allows cleartext HTTP to `127.0.0.1`. Debug builds of an app that embeds this plugin also allow `10.0.2.2` (and `localhost`). Release builds are unchanged.

### Android (Dioxus CLI 0.7+)

1. Depend on this crate (`path` / git).
2. Build with **Dioxus CLI 0.7+** (`dx`). The crate ships a Gradle library under [`android/`](android/) and emits manganis Android artifact metadata so `dx` embeds the Kotlin host automatically — **no copy/paste of Kotlin**.
3. The plugin module depends on `com.google.firebase:firebase-auth` via the Firebase BoM.

### iOS app deps

1. Add [Firebase iOS SDK](https://github.com/firebase/firebase-ios-sdk) via Swift Package Manager and link **FirebaseAuth** (and **FirebaseCore**).
2. Compile [`ios/DioxusFirebaseAuthHost.swift`](ios/DioxusFirebaseAuthHost.swift) from this crate into your Dioxus iOS target (copy or path reference in Xcode / your `dx` mobile project).

## Testing

Host checks (`UnsupportedPlatform` on desktop) live in [`tests/unsupported/mod.rs`](tests/unsupported/mod.rs):

```bash
cargo test
```

Android `--lib` tests use the same target as [`.github/workflows/test.yml`](.github/workflows/test.yml). They run without a Kotlin host and expect `HostMissing`. They do not execute [`tests/android/mod.rs`](tests/android/mod.rs).

```bash
# cross — builds and runs under QEMU (CI)
cross test --lib --target aarch64-linux-android

# cargo-ndk — same tests with a local NDK
rustup target add aarch64-linux-android
cargo ndk -t arm64-v8a test --lib
```

The on-device Auth lifecycle test needs an app process with the Kotlin host. A raw `cargo` or `cross` test process hits `HostMissing` and returns early.

## Demo

[`examples/demo.rs`](examples/demo.rs) signs in against the Auth emulator at `10.0.2.2:9099` (see [Emulator](#emulator); no `adb reverse`). Start the emulator, then serve the example on Android:

```bash
npx -y firebase-tools emulators:start --only auth
dx serve --example demo --platform android
```

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.
