# Google sign-in

Same Rust APIs on both platforms (`sign_in_with_google` / `link_with_google`). Enable Google under Authentication in the Firebase Console. Pass the OAuth **web** client ID as `google_web_client_id` on [`FirebaseOptions`](getting-started.md).

Wire codes include `cancelled` (sheet dismissed) and `unavailable` (missing client ID / Activity / presentation). Federated sign-in against the Auth emulator is limited — keep email/emulator for CI; exercise live Google on a real project.

Apple Sign-In host code remains in the Android/iOS hosts but is **not exported** from the Rust API yet (blocked — re-enable later).

## Android

- Register the app’s **debug SHA-1** in Firebase Console for the Android app whose `GOOGLE_APP_ID` you pass. Set `ANDROID_APPLICATION_ID` and serve with `make android-demo` (syncs the package).
- Google uses Credential Manager with `google_web_client_id` as `serverClientId` (Play services on device; `minSdk` stays 24). The host tries One Tap (`GetGoogleIdOption`), then falls back to the Sign in with Google button flow (`GetSignInWithGoogleOption`) when no matching credential is found.
- `sign_out` clears Firebase Auth and best-effort Credential Manager state.

For a live demo APK, add the **built APK’s** SHA-1 (`apksigner verify --print-certs …/app-debug.apk`) — `dx` may not use `~/.android/debug.keystore`. See [Demo and Maestro](demo-and-maestro.md).

## iOS

- Pass `google_web_client_id` and/or set `GIDClientID` in Info.plist; add the reversed client ID URL scheme.
- The **app** must forward open-URL callbacks to `GIDSignIn.sharedInstance.handle(url)` (scene / app delegate — Dioxus owns that path).
- `sign_out` clears Firebase Auth and best-effort `GIDSignIn` session state.

Linking Google to an existing user: [Errors and linking](errors-and-linking.md).
