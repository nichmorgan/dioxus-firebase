# Getting started

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
    // Crate extension (not a core FirebaseOptions field): OAuth *web* client ID
    // from Firebase Console → Project settings → Your apps (or Google Cloud
    // credentials). Required for Google sign-in / link.
    google_web_client_id: std::env::var("FIREBASE_GOOGLE_WEB_CLIENT_ID").ok(),
})?;
```

Double-init returns `AlreadyInitialized`. Desktop / web builds return `UnsupportedPlatform`.

Call mutations (`sign_in_with_email_and_password`, `sign_in_with_google`, …) from `spawn_blocking` (or another worker thread) — not Dioxus `spawn`, which is often still the UI thread on iOS. Social flows present a system sheet on the main thread, then wait on the worker for the credential and Firebase Task.

Do **not** put `google-services.json` keys inside this library. The app passes options at init. See the [ownership table](../README.md#ownership) in the README.

## Android (Dioxus CLI 0.7+)

1. Depend on this crate (`path` / git).
2. Build with **Dioxus CLI 0.7+** (`dx`). The crate ships a Gradle library under [`android/`](../android/) and emits manganis Android artifact metadata so `dx` embeds the Kotlin host automatically — **no copy/paste of Kotlin**.
3. The plugin module depends on `com.google.firebase:firebase-auth` via the Firebase BoM, plus AndroidX Credential Manager / Google Identity for Google sign-in.

`HostMissing` means this process was not built with `dx`, so the Kotlin host was never loaded. A raw `cargo` or `cross` test process is not an app process — see [Testing](testing.md).

Google Console / SHA-1 setup: [Google sign-in](google-sign-in.md).

## iOS (Dioxus CLI 0.7+)

1. Depend on this crate (`path` / git).
2. Build with **Dioxus CLI 0.7+** (`dx`). The crate ships a Swift package under [`ios/`](../ios/) and emits manganis Swift package metadata so `dx` compiles and embeds the host automatically — **no copy/paste of Swift**.
3. The package depends on **FirebaseAuth**, **FirebaseCore** (Firebase iOS SDK 11.15.0), and **GoogleSignIn**.

`HostMissing` means this process was not built with `dx`, so the Swift framework was never loaded. A non-`dx` build can add the local package [`ios/`](../ios/) in Xcode (product `DioxusFirebaseAuthHost`); that package already depends on FirebaseAuth, FirebaseCore, and GoogleSignIn.

Firebase Auth stores the signed-in user in the keychain. A `dx` simulator build without `--codesign` has no `application-identifier` or `keychain-access-groups` entitlement, so sign-in fails with `keychain-error` (`SecItemAdd` -34018). The demo embeds those entitlements in the simulator binary ([`examples/demo-simulator.entitlements`](../examples/demo-simulator.entitlements), section `__TEXT,__entitlements`). Apps should do the same, or build with `--codesign` and an Apple Development identity. This crate does not inject the section; entitlements belong to the app.

Google URL schemes and open-URL forwarding: [Google sign-in](google-sign-in.md).
