## Learned User Preferences

- Shape this crate like the sibling `dioxus-qonversion` library and the Dioxus org library template: `dioxus-lib` as a dependency, `dioxus` as a dev-dependency, and `.github/` plus `examples/` adapted from that template.
- Write Rust tests with `rstest`, including native Android/iOS parity tests.
- Keep end-to-end coverage in this crate as normal Rust tests. Run Android tests with `cross test` (or `cargo ndk`).
- Serve and document the demo only with `dx serve --example demo`. Start a stateless Auth emulator with `make emulator` (Firebase CLI, project `demo-project`, no import/export), using `10.0.2.2` on the Android emulator. Maestro does not delete accounts.
- Keep GitHub Actions short by reusing trusted existing actions. Maestro UI tests are an Android and iOS matrix. Suite order is `.maestro/config.yaml` `flowsOrder`: signup, then login, with `continueOnFailure: false`.
- Leave `Cargo.lock` gitignored. This is a library, and downstream apps resolve their own lockfile.
- Keep dependency source out of this library. A vendored tree such as `third_party/tao`, or a path `[patch]` that requires it, does not belong here.

## Learned Workspace Facts

- `dioxus-firebase` is a mobile-only (Android and iOS) Dioxus bridge for Firebase Auth email/password. Web and desktop return `UnsupportedPlatform`. Dioxus CLI 0.7+ embeds the Kotlin and Swift hosts via manganis, so apps do not copy those hosts.
- Load `io.dioxus.firebase.DioxusFirebaseAuthHost` only through the Activity `ClassLoader.loadClass` path. `Env::find_class` from `attach_current_thread` panics on release/arm64 when a JNI exception is pending; clear that exception before `getClassLoader`. `src/native/android.rs` is `cfg(target_os = "android")`, so check it with `--target aarch64-linux-android` or `cross`.
- The iOS host tracks the Kotlin host (shared Firebase app and Auth getters). Firebase iOS SDK 11.x uses `AuthErrorCode(rawValue:)`.
- A plain `cargo` or `cross` test process is not the app process, so live Auth calls return `HostMissing`. On-device Auth flows need a `dx` build (the demo or Maestro). iOS `--lib` tests on `aarch64-apple-ios-sim` do not run `tests/ios`.
- Debug Android builds allow cleartext to the Auth emulator at `10.0.2.2` and `127.0.0.1`. Release network security does not. The demo uses `10.0.2.2` when `ro.boot.qemu=1` or the hardware is goldfish/ranchu, and `127.0.0.1` on a physical Android device or the iOS simulator. A USB device needs `adb reverse tcp:9099 tcp:9099`.
- `make emulator` stops whatever is listening on port 9099, then runs `emulators:start` in the foreground for project `demo-project` with no import or export. `make maestro` uses `emulators:exec` with one quoted script so Maestro starts only after Auth is listening and the emulator exits with the suite. Accounts exist only in that process.
- Maestro flows split with `when.platform`. Android waits with `childOf` `auth-status` and taps by `id` plus text. A selector that requires the id and the status text on one node times out. iOS does not expose those ids, so it waits and taps by text only (`Firebase Auth initialized & subscribed.`, `Sign Up`, `Sign In`, `Update Display Name`). `assertVisible` matches the full accessibility string, so status lines that include a uid use a regex (`Created user:.*`, `Signed in user:.*`).
- Firebase iOS `FIRApp.configure` raises an `NSException` when `GOOGLE_APP_ID` is not `version:number:ios:<hex>`. Swift `do/catch` does not catch it, so the process aborts. The Swift host rejects that shape and returns `invalid-app-id` before configure. The demo uses `1:123456789:ios:0123456789abcdef` on iOS and `1:123456789:android:demo` on Android.
- The Android step in `.github/workflows/ui.yml` uses `reactivecircus/android-emulator-runner`, which runs each `script` line as a separate `/usr/bin/sh -c`. The script cannot use multi-line constructs or shell state that carries across lines. The APK is located in an earlier bash step and exported through `GITHUB_ENV`.
