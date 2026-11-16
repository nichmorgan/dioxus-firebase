# Demo and Maestro

[`examples/demo.rs`](../examples/demo.rs) is a home grid of feature tiles (Email & Password, Google, Account). Apple is hidden until Sign-In is re-enabled.

The tracked template [`Dioxus.toml.example`](../Dioxus.toml.example) stays generic (`com.example.demo`); `make sync-bundle-id` (via `make *-demo`) writes a gitignored root `Dioxus.toml` for `dx` (required because the demo is a Cargo example of the root package). Package and Firebase options are parameterized from a local `.env` — `cp .env.example .env` and choose a mode.

## Emulator mode (email / Maestro)

Leave project keys empty (or set `FIREBASE_USE_AUTH_EMULATOR=1`). Leave `ANDROID_APPLICATION_ID` empty. `make android-demo` / `make ios-demo` generate `Dioxus.toml` with `com.example.demo` and serve. The demo uses `demo-project` and talks to the Auth emulator at `10.0.2.2:9099` on the Android emulator, or `127.0.0.1:9099` on a physical Android device / iOS simulator (see [Auth emulator](auth-emulator.md)).

```bash
# Emulator mode only — skip make emulator / adb reverse when FIREBASE_USE_AUTH_EMULATOR=0
adb reverse tcp:9099 tcp:9099
make emulator
make android-demo   # sync-bundle-id then dx serve --example demo --platform android
make ios-demo       # sync-bundle-id then dx serve --example demo --platform ios
```

## Live mode (real Firebase)

Set `FIREBASE_API_KEY`, `FIREBASE_APP_ID` (platform-matched `…:android:…` or `…:ios:…`), `FIREBASE_PROJECT_ID`, and `FIREBASE_GOOGLE_WEB_CLIENT_ID`, then set `FIREBASE_USE_AUTH_EMULATOR=0`. Set `ANDROID_APPLICATION_ID` to **your** Firebase Android app package (must match `FIREBASE_APP_ID`). Serve with `make android-demo` / `make ios-demo` (they generate local `Dioxus.toml` first).

Checklist:

- Enable Email / Google under Authentication in the Firebase Console
- Android: add the **built APK’s** SHA-1 (`apksigner verify --print-certs …/app-debug.apk`) on the Android app that matches `FIREBASE_APP_ID` / `ANDROID_APPLICATION_ID` (`dx` may not use `~/.android/debug.keystore`)
- Empty `ANDROID_APPLICATION_ID` + `make android-demo` / `make ios-demo` restores the default package for Maestro / CI. Maestro flows use `appId: ${ANDROID_APPLICATION_ID}` (passed by `make maestro`); do not put a package name in `.maestro/*.yaml`

## Maestro

[`.maestro/`](../.maestro/) opens the Email & Password tile, signs up with the prefilled account, then logs in with it. The suite does not delete accounts. A re-run needs a new `make emulator` or `make maestro`, because nothing clears accounts inside a running emulator.

`make maestro` starts an empty Auth emulator, runs both flows against that process, and shuts it down. [`.github/workflows/ui.yml`](../.github/workflows/ui.yml) runs both flows on an Android emulator and an iOS simulator. Locally, with the demo installed on a booted device:

```bash
# USB Android device only. The Android emulator reaches the host at 10.0.2.2.
adb reverse tcp:9099 tcp:9099
make maestro
```
