# dioxus-firebase

**Unofficial** [Dioxus](https://dioxuslabs.com/) bridge for [Firebase Auth](https://firebase.google.com/docs/auth) (email/password, Google, Apple).

**Mobile only for now** (iOS and Android). Web and desktop return `UnsupportedPlatform`.

This crate is **not affiliated** with Google / Firebase. [JusFine](https://github.com/nichmorgan/jusfine-ui-dioxus) dogfoods it for production auth.

## Status

Early — email/password plus Google and Apple sign-in/link/unlink, profile display name / photo URL. No phone auth, email verification, or custom claims.

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

Index: [docs/README.md](docs/README.md).

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.
