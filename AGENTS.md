## Learned User Preferences

- Shape this crate like the sibling `dioxus-qonversion` library and the Dioxus org library template: `dioxus-lib` as a dependency, `dioxus` as a dev-dependency, and `.github/` plus `examples/` adapted from that template.
- Write Rust tests with `rstest`, including native Android/iOS parity tests.
- Keep end-to-end coverage in this crate as normal Rust tests. Run Android tests with `cross test` (or `cargo ndk`).
- Serve and document the demo only with `dx serve --example demo`. Start a stateless Auth emulator with `make emulator` (Firebase CLI, project `demo-project`, no import/export), using `10.0.2.2` on the Android emulator. Maestro does not delete accounts.
- Keep GitHub Actions short by reusing trusted existing actions. Maestro UI tests are an Android and iOS matrix: signup first, then login.
- Leave `Cargo.lock` gitignored. This is a library, and downstream apps resolve their own lockfile.

## Learned Workspace Facts

- `dioxus-firebase` is a mobile-only (Android and iOS) Dioxus bridge for Firebase Auth email/password. Web and desktop return `UnsupportedPlatform`. Dioxus CLI 0.7+ embeds the Kotlin and Swift hosts via manganis, so apps do not copy those hosts.
- Load `io.dioxus.firebase.DioxusFirebaseAuthHost` only through the Activity `ClassLoader.loadClass` path. `Env::find_class` from `attach_current_thread` panics on release/arm64 when a JNI exception is pending; clear that exception before `getClassLoader`. `src/native/android.rs` is `cfg(target_os = "android")`, so check it with `--target aarch64-linux-android` or `cross`.
- The iOS host tracks the Kotlin host (shared Firebase app and Auth getters). Firebase iOS SDK 11.x uses `AuthErrorCode(rawValue:)`.
- A plain `cargo` or `cross` test process is not the app process, so live Auth calls return `HostMissing`. On-device Auth flows need a `dx` build (the demo or Maestro). iOS `--lib` tests on `aarch64-apple-ios-sim` do not run `tests/ios`.
- Debug Android builds allow cleartext to the Auth emulator at `10.0.2.2`. Release network security does not. The iOS simulator uses `127.0.0.1`.
- `make emulator` and `make maestro` each start a new Auth emulator for project `demo-project` with no import or export. Accounts exist only in that process.
