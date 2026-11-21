# Errors and linking

Failures return `FirebaseError::AuthException { code, message }` with Firebase-style wire codes such as:

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
- `no-such-provider` (unlink of a provider that is not linked)
- `no-current-user` (link, unlink, or profile update while signed out)
- `requires-recent-login` (sensitive op needs a fresh sign-in)
- `operation-not-allowed` (a second iOS Apple sign-in or link while a sheet is already up)
- `canceled` (user dismissed the Google sheet, or the iOS Apple sheet)
- `web-context-canceled` (user dismissed the Android Apple Custom Tab)
- `web-context-already-presented` (Android Apple Custom Tab closed early; often `android:taskAffinity=""`)
- `unavailable` (missing client ID / Activity / presentation context)

Map codes to user-facing strings in the app (anti-enumeration on login, etc.).

## Link / unlink (no switch API)

- `link_with_google` — link Google to the current user
- `link_with_apple` — link Apple to the current user
- `unlink` — pass a provider id (`"google.com"`, `"apple.com"`, `"password"`, …)

Firebase allows one login per provider, so the client cannot link a second Google or Apple identity next to the first. `unlink("google.com")` then `link_with_google()` attaches a different identity to the same uid, but a dismissed sheet or a failed link leaves the user without that login (and with no login at all when it was the only one).

To switch without that gap, call `pick_google_credential()` or `pick_apple_credential()` (iOS only). They open the same sheets and return the ID token without touching Firebase, so a dismiss changes nothing. Send the token to your backend, verify it, and swap the provider there with Identity Toolkit `accounts:update` (`deleteProvider` plus `linkProviderUserInfo`). Then call `reload_current_user()` so the client sees the new `providerData`.

Signing in with that provider after unlink (without linking) creates a **new** Firebase user (Firebase Auth behavior).

Google Console and platform setup: [Google sign-in](google-sign-in.md). Apple, including the same uid on Android and iOS: [Apple sign-in](apple-sign-in.md).
