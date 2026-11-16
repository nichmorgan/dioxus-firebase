# Testing

Host checks (`UnsupportedPlatform` on desktop) live in [`tests/unsupported/mod.rs`](../tests/unsupported/mod.rs):

```bash
cargo test
```

## Android

Android `--lib` tests use the same target as [`.github/workflows/test.yml`](../.github/workflows/test.yml). They run without a Kotlin host and expect `HostMissing`. They do not execute [`tests/android/mod.rs`](../tests/android/mod.rs).

```bash
# cross — builds and runs under QEMU (CI)
cross test --lib --target aarch64-linux-android

# cargo-ndk — same tests with a local NDK
rustup target add aarch64-linux-android
cargo ndk -t arm64-v8a test --lib
```

The on-device Auth lifecycle test needs an app process with the Kotlin host. A raw `cargo` or `cross` test process hits `HostMissing` and returns early.

## iOS

iOS `--lib` tests use the same simulator target as CI. They run without the Swift host and expect `HostMissing`. They do not execute [`tests/ios/mod.rs`](../tests/ios/mod.rs).

```bash
rustup target add aarch64-apple-ios-sim
cargo test --lib --target aarch64-apple-ios-sim
```

On a Mac, boot a simulator first (CI does this with `xcrun simctl`) or set `CARGO_TARGET_AARCH64_APPLE_IOS_SIM_RUNNER` to `xcrun simctl spawn booted`.

UI / on-device Auth flows: [Demo and Maestro](demo-and-maestro.md).
