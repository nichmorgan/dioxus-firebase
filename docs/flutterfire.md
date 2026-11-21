# FlutterFire

This crate is inspired by [FlutterFire](https://github.com/firebase/flutterfire) `firebase_auth`. The Android host follows `FlutterFirebaseAuthPlugin`. The iOS host follows `FLTFirebaseAuthPlugin`, including the Apple path that uses `ASAuthorization` plus `OAuthProvider.appleCredential` because Firebase Auth 11.15 fatalErrors `OAuthProvider(providerID: "apple.com")` off the emulator.

An extra exists only when an app cannot get the same result by calling the public API. The caller-facing map is in the [README](../README.md#flutterfire). This page adds the host names and the platform notes.

## Covered

| Rust | Dart | Kotlin | Swift |
| -- | -- | -- | -- |
| `initialize_app` | `Firebase.initializeApp` | `initializeApp` | `initializeAppWithApiKey:appId:projectId:messagingSenderId:storageBucket:databaseURL:googleWebClientId:` |
| `sign_in_with_email_and_password` | `FirebaseAuth.signInWithEmailAndPassword` | `signInWithEmailAndPassword` | `signInWithEmailAndPassword:password:` |
| `create_user_with_email_and_password` | `FirebaseAuth.createUserWithEmailAndPassword` | `createUserWithEmailAndPassword` | `createUserWithEmailAndPassword:password:` |
| `update_display_name` | `User.updateDisplayName` | `updateDisplayName` | `updateDisplayName:` |
| `update_photo_url` | `User.updatePhotoURL` | `updatePhotoURL` | `updatePhotoURL:` |
| `send_password_reset_email` | `FirebaseAuth.sendPasswordResetEmail` | `sendPasswordResetEmail` | `sendPasswordResetEmail:` |
| `sign_out` | `FirebaseAuth.signOut` | `signOut` | `signOut` |
| `current_user` | `FirebaseAuth.currentUser` | `currentUser` | `currentUser` |
| `get_id_token` | `User.getIdToken` | `getIdToken` | `getIdTokenWithForceRefresh:` |
| `use_auth_emulator` | `FirebaseAuth.useAuthEmulator` | `useAuthEmulator` | `useAuthEmulatorWithHost:port:` |
| `auth_state_changes` | `FirebaseAuth.authStateChanges` | `authStateChanges` | `authStateChangesWithCallback:` |
| `unlink` | `User.unlink` | `unlink` | `unlink:` |
| `sign_in_with_google` | `FirebaseAuth.signInWithProvider(GoogleAuthProvider())` | `signInWithGoogle` | `signInWithGoogle` |
| `link_with_google` | `User.linkWithProvider(GoogleAuthProvider())` | `linkWithGoogle` | `linkWithGoogle` |
| `sign_in_with_apple` | `FirebaseAuth.signInWithProvider(AppleAuthProvider())` | `signInWithApple` | `signInWithApple` |
| `link_with_apple` | `User.linkWithProvider(AppleAuthProvider())` | `linkWithApple` | `linkWithApple` |
| `delete_current_user` | `User.reauthenticateWithProvider` plus `User.delete` | `deleteCurrentUser` | `deleteCurrentUser:` |

FlutterFire has no `signInWithGoogle` or `signInWithApple`. Those Dart cells are the generic provider calls.

## Not wrapped

FlutterFire Auth methods this crate does not expose:

- `signInAnonymously`, `signInWithCustomToken`, `signInWithEmailLink`
- `signInWithCredential`, `linkWithCredential`, `reauthenticateWithCredential`, and a standalone `reauthenticateWithProvider`
- `sendEmailVerification`, `verifyBeforeUpdateEmail`, `updateEmail`, `updatePassword`
- `User.delete` without reauth (use `delete_current_user`)
- `applyActionCode`, `checkActionCode`, `confirmPasswordReset`, `verifyPasswordResetCode`
- Phone auth and multi-factor (including TOTP)
- `idTokenChanges`, `userChanges`
- Other providers: Facebook, GitHub, Microsoft, Game Center, and generic OAuth
- `additionalUserInfo.authorizationCode` and a standalone `revokeTokenWithAuthorizationCode`. `delete_current_user("apple.com")` revokes internally. FlutterFire’s Android `revokeTokenWithAuthorizationCode` returns success and does not revoke.

## Extras

`FirebaseOptions.google_web_client_id` is the Google server client id. Android Credential Manager uses it as the server client id. On iOS it overrides `GIDClientID` from Info.plist. FlutterFire reads that id from the Google Sign-In plugin and `google-services` config instead of from `FirebaseOptions`.

`sign_in_with_google` and `link_with_google` open a fixed Google sheet: Credential Manager on Android (One Tap, then the Sign in with Google button when One Tap has no credential), and `GIDSignIn` on iOS. FlutterFire’s generic `signInWithProvider(GoogleAuthProvider())` is a different native path.

`sign_in_with_apple` and `link_with_apple` are fixed Apple entry points.

- Android builds `OAuthProvider("apple.com")` with scopes `email` and `name`, then calls `startActivityForSignInWithProvider` or `startActivityForLinkWithProvider`. Each call starts that task. The host does not resume `pendingAuthResult`.
- iOS uses `ASAuthorization` (scopes email and name, SHA-256 nonce) and `OAuthProvider.appleCredential(withIDToken:rawNonce:fullName:)`, then `signIn(with:)` or `link(with:)`. That credential applies the first-sign-in name, the same way FlutterFire does.
- A second iOS Apple sign-in or link while a sheet or its Firebase follow-up is still running returns `operation-not-allowed` with `A Sign in with Apple request is already in progress.` Android does not have that guard. FlutterFire’s Android plugin does not either.

On Android, if Firebase leaves `displayName` blank after Apple auth, the host copies a name once from `additionalUserInfo.profile` (`name`, `fullName`, `full_name`, or `given_name` plus `family_name`). Custom Tabs have no `fullName`, and Apple sends the name only on the first authorization for that Services ID. iOS does not do this copy. `appleCredential(fullName:)` is what stores the name there.

`delete_current_user` takes `"google.com"` or `"apple.com"`. Google calls `delete` first and reauthenticates only on `requires-recent-login`: Android uses `GetGoogleIdOption` with `setFilterByAuthorizedAccounts(true)` and `setAutoSelectEnabled(true)`, falling back to the sign-in picker when no account is authorized; iOS uses `GIDSignIn` `currentUser` or `restorePreviousSignIn` when its email matches the Firebase `google.com` provider, then `refreshTokensIfNeeded`, else `signIn(withPresenting:hint:)` with that email. It then calls `reauthenticate` and `delete`. Apple opens its sheet before any Firebase call. Apple on Android uses `startActivityForReauthenticateWithProvider`, then `revokeAccessToken` with the `OAuthCredential` access token. Apple on iOS keeps the `ASAuthorizationAppleIDCredential` `authorizationCode`, reauthenticates with `appleCredential`, then calls `revokeToken(withAuthorizationCode:)`. A failed revoke still deletes; a failed reauth or delete does not.

`sign_out` calls Firebase `signOut`, then best-effort clears Android Credential Manager and calls `GIDSignIn.sharedInstance.signOut()` on iOS. A failure in those clears does not fail the Firebase sign-out.

`is_initialized` reports whether `initialize_app` has succeeded in this process. FlutterFire has no matching call.

## Differences kept on purpose

Link collisions return `credential-already-in-use` or `account-exists-with-different-credential`. The host does not sign in with `updatedCredential`, and the wire error does not carry that credential. FlutterFire attaches it to the error so the app can call `signInWithCredential`. This crate has no credential API. To use the other account, call `sign_in_with_google` or `sign_in_with_apple`. To keep the current uid, `unlink` the provider and link again.

Signed-out `link_with_apple`, `unlink`, `delete_current_user`, and profile updates return `no-current-user` with `No user currently signed in.` before a sheet opens. `link_with_google` obtains the Google credential first, then returns `no-current-user` if nobody is signed in. `requires-recent-login` stays the Firebase code for a signed-in user whose session is too old.

`get_id_token` returns `Ok(None)` when signed out. FlutterFire’s `User.getIdToken` is called on a user and errors with `no-current-user` when `currentUser` is missing. Call `current_user` first when the app needs to tell “signed out” from “no token”.
