# Google sign-in

Same Rust APIs on both platforms (`sign_in_with_google` / `link_with_google`). Enable Google under Authentication in the Firebase Console. Pass the OAuth **web** client ID as `google_web_client_id` on [`FirebaseOptions`](getting-started.md).

Wire codes include `canceled` (sheet dismissed) and `unavailable` (missing client ID / Activity / presentation). Federated sign-in against the Auth emulator is limited — keep email/emulator for CI; exercise live Google on a real project.

Apple: [Apple sign-in](apple-sign-in.md).

## Android

- Register the app’s **debug SHA-1** in Firebase Console for the Android app whose `GOOGLE_APP_ID` you pass. Set `ANDROID_APPLICATION_ID` and serve with `make android-demo` (syncs the package).
- Google uses Credential Manager with `google_web_client_id` as `serverClientId` (Play services on device; `minSdk` stays 24). The host tries One Tap (`GetGoogleIdOption`), then falls back to the Sign in with Google button flow (`GetSignInWithGoogleOption`) when no matching credential is found.
- `sign_out` clears Firebase Auth and best-effort Credential Manager state.

For a live demo APK, add the **built APK’s** SHA-1 (`apksigner verify --print-certs …/app-debug.apk`) — `dx` may not use `~/.android/debug.keystore`. See [Demo and Maestro](demo-and-maestro.md).

## iOS

- Set `GIDClientID` in Info.plist to the OAuth **iOS** client id (bundle id of this app). Add that client's reversed id as a URL scheme (`123-abc.apps.googleusercontent.com` → `com.googleusercontent.apps.123-abc`).
- Pass `google_web_client_id` as well. The host uses it as `serverClientID`. A web client cannot be `clientID` on iOS: Google rejects the custom scheme with `invalid_request` (`Custom scheme URIs are not allowed for 'WEB' client type`).
- If `GIDClientID` is absent, the host falls back to `google_web_client_id` as `clientID`. That path cannot finish a custom-scheme sign-in.
- The **app** must forward open-URL callbacks to `GIDSignIn.sharedInstance.handle(url)` (scene / app delegate — Dioxus owns that path).
- `sign_out` clears Firebase Auth and best-effort `GIDSignIn` session state.

Linking Google to an existing user: [Errors and linking](errors-and-linking.md).
