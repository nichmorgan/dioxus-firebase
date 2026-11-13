# dioxus-firebase

**Unofficial** [Dioxus](https://dioxuslabs.com/) bridge for [Firebase Auth](https://firebase.google.com/docs/auth) (email/password, Google, Apple).

**Mobile only for now** (iOS and Android). Web and desktop return `UnsupportedPlatform`.

This crate is **not affiliated** with Google / Firebase. [JusFine](https://github.com/nichmorgan/jusfine-ui-dioxus) dogfoods it for production auth.

## Status

Early — email/password plus Google and Apple sign-in/link. No phone auth, email verification, or custom claims.

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
| **App** | `FirebaseOptions` (api key, app id, project id, optional Google web client ID, staging vs prod), Console provider enablement, Android SHA-1, iOS Sign in with Apple capability / Google URL schemes, PT-BR / localized copy, route gates |

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
    // OAuth *web* client ID from Firebase Console → Project settings → Your apps
    // (or Google Cloud credentials). Required for Google sign-in / link.
    google_web_client_id: std::env::var("FIREBASE_GOOGLE_WEB_CLIENT_ID").ok(),
})?;
```

Double-init returns `AlreadyInitialized`. Desktop / web builds return `UnsupportedPlatform`.

Call mutations (`sign_in_with_email`, `sign_in_with_google`, …) from `spawn_blocking` (or another worker thread) — not Dioxus `spawn`, which is often still the UI thread on iOS. Social flows present a system sheet on the main thread, then wait on the worker for the credential and Firebase Task.

### Google / Apple app setup

Same Rust APIs on both platforms (`sign_in_with_google` / `sign_in_with_apple` / `link_with_*`). Enable Google and/or Apple under Authentication in the Firebase Console. Pass the OAuth **web** client ID as `google_web_client_id` when using Google. Wire codes include `cancelled` (sheet dismissed) and `unavailable` (missing client ID / Activity / presentation). Federated sign-in against the Auth emulator is limited — keep email/emulator for CI; exercise live Google/Apple on a real project.

**Android**

- Register the app’s SHA-1 in Firebase Console (Google and Apple).
- Google uses Credential Manager with `google_web_client_id` as `serverClientId` (Play services on device; `minSdk` stays 24).
- Apple uses Firebase-managed Custom Tabs (`OAuthProvider`). Configure an Apple Services ID, return URL `https://PROJECT_ID.firebaseapp.com/__/auth/handler`, and OAuth code-flow keys (Team ID, key, key ID) in Console. The host checks `pendingAuthResult` before starting a new Custom Tab flow if the Activity was reclaimed mid-login.
- `sign_out` clears Firebase Auth and best-effort Credential Manager state.

**iOS**

- Google: pass `google_web_client_id` and/or set `GIDClientID` in Info.plist; add the reversed client ID URL scheme. The **app** must forward open-URL callbacks to `GIDSignIn.sharedInstance.handle(url)` (scene / app delegate — Dioxus owns that path).
- Apple: enable the Sign in with Apple capability / entitlements on the **app**. The host generates the nonce and builds `OAuthProvider.appleCredential` (including `fullName` on first sign-in).
- `sign_out` clears Firebase Auth and best-effort `GIDSignIn` session state.

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
- `account-exists-with-different-credential`
- `credential-already-in-use`
- `provider-already-linked`
- `cancelled` (user dismissed the system sheet)
- `unavailable` (missing client ID / Activity / presentation context)

`User.provider_ids` lists linked providers (`password`, `google.com`, `apple.com`, …) on every snapshot, including auth-state callbacks.

Map codes to user-facing strings in the app (anti-enumeration on login, etc.).

## Emulator

Optional: `use_emulator(host, port)` after init. JusFine can keep using staging cloud in debug and adopt the emulator later.

On the Android emulator, call `use_emulator("10.0.2.2", 9099)`. `10.0.2.2` is the emulator's alias for the host machine, so no `adb reverse` is required. A USB-connected Android device cannot use that alias. Call `use_emulator("127.0.0.1", 9099)` and forward the Auth port over the adb connection (re-run it after the device reconnects):

```bash
adb reverse tcp:9099 tcp:9099
```

The iOS simulator shares the host network, so call `use_emulator("127.0.0.1", 9099)`. The demo uses `10.0.2.2` on an Android emulator and `127.0.0.1` on a physical Android device or the iOS simulator. Start a stateless Auth emulator on the host (`0.0.0.0:9099` in [`firebase.json`](firebase.json)). `make emulator` stops anything already listening on port 9099, then starts a new process with an empty account store. It does not import or export accounts:

```bash
make emulator
```

Accounts created while that process is running stay until it exits. The next `make emulator` does not load them.

Dioxus CLI's generated network-security config only allows cleartext HTTP to `127.0.0.1`. Debug builds of an app that embeds this plugin also allow `10.0.2.2` (and `localhost`). Release builds are unchanged.

### Android (Dioxus CLI 0.7+)

1. Depend on this crate (`path` / git).
2. Build with **Dioxus CLI 0.7+** (`dx`). The crate ships a Gradle library under [`android/`](android/) and emits manganis Android artifact metadata so `dx` embeds the Kotlin host automatically — **no copy/paste of Kotlin**.
3. The plugin module depends on `com.google.firebase:firebase-auth` via the Firebase BoM, plus AndroidX Credential Manager / Google Identity for Google sign-in.

### iOS (Dioxus CLI 0.7+)

1. Depend on this crate (`path` / git).
2. Build with **Dioxus CLI 0.7+** (`dx`). The crate ships a Swift package under [`ios/`](ios/) and emits manganis Swift package metadata so `dx` compiles and embeds the host automatically — **no copy/paste of Swift**.
3. The package depends on **FirebaseAuth**, **FirebaseCore** (Firebase iOS SDK 11.15.0), and **GoogleSignIn**.

`HostMissing` means this process was not built with `dx`, so the Swift framework was never loaded. A non-`dx` build can add the local package [`ios/`](ios/) in Xcode (product `DioxusFirebaseAuthHost`); that package already depends on FirebaseAuth, FirebaseCore, and GoogleSignIn.

Firebase Auth stores the signed-in user in the keychain. A `dx` simulator build without `--codesign` has no `application-identifier` or `keychain-access-groups` entitlement, so sign-in fails with `keychain-error` (`SecItemAdd` -34018). The demo embeds those entitlements in the simulator binary ([`examples/demo-simulator.entitlements`](examples/demo-simulator.entitlements), section `__TEXT,__entitlements`). Apps should do the same, or build with `--codesign` and an Apple Development identity. This crate does not inject the section; entitlements belong to the app.

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

iOS `--lib` tests use the same simulator target as CI. They run without the Swift host and expect `HostMissing`. They do not execute [`tests/ios/mod.rs`](tests/ios/mod.rs).

```bash
rustup target add aarch64-apple-ios-sim
cargo test --lib --target aarch64-apple-ios-sim
```

On a Mac, boot a simulator first (CI does this with `xcrun simctl`) or set `CARGO_TARGET_AARCH64_APPLE_IOS_SIM_RUNNER` to `xcrun simctl spawn booted`.

## Demo

[`examples/demo.rs`](examples/demo.rs) talks to the Auth emulator at `10.0.2.2:9099` on the Android emulator, and at `127.0.0.1:9099` on a physical Android device or the iOS simulator (see [Emulator](#emulator)). Start the emulator, then serve the example. On a USB Android device, reverse the Auth port first:

```bash
adb reverse tcp:9099 tcp:9099
make emulator
dx serve --example demo --platform android
dx serve --example demo --platform ios
```

[`.maestro/`](.maestro/) signs up with the prefilled account, then logs in with it. The suite does not delete accounts. A re-run needs a new `make emulator` or `make maestro`, because nothing clears accounts inside a running emulator. `make maestro` starts an empty Auth emulator, runs both flows against that process, and shuts it down. [`.github/workflows/ui.yml`](.github/workflows/ui.yml) runs both flows on an Android emulator and an iOS simulator. Locally, with the demo installed on a booted device:

```bash
# USB Android device only. The Android emulator reaches the host at 10.0.2.2.
adb reverse tcp:9099 tcp:9099
make maestro
```

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.
