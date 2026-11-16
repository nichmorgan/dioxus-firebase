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
- `requires-recent-login` (sensitive op needs a fresh sign-in)
- `cancelled` (user dismissed the system sheet)
- `unavailable` (missing client ID / Activity / presentation context)

Map codes to user-facing strings in the app (anti-enumeration on login, etc.).

## Link / unlink (no switch API)

- `link_with_google` — link Google to the current user
- `unlink` — pass a provider id (`"google.com"`, `"apple.com"`, `"password"`, …)

There is **no** dedicated “switch store account” helper. To attach a different Google identity to the same Firebase uid: `unlink("google.com")` then `link_with_google()` while still signed in. Signing in with that provider after unlink (without linking) creates a **new** Firebase user (Firebase Auth behavior).

Google Console and platform setup: [Google sign-in](google-sign-in.md).
