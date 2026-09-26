//! Non-mobile host platforms integration test target.
#![cfg(not(any(target_os = "android", target_os = "ios")))]

use super::common::demo_options;
use dioxus_firebase::prelude::*;
use rstest::rstest;

#[rstest]
fn test_desktop_returns_unsupported_platform(demo_options: FirebaseOptions) {
    let err = initialize(demo_options).expect_err("desktop must fail with UnsupportedPlatform");
    assert_eq!(err, FirebaseError::UnsupportedPlatform);
}

#[rstest]
fn test_primitives_require_init_on_desktop() {
    assert_eq!(
        sign_in_with_email("a@b.c", "pass").unwrap_err(),
        FirebaseError::NotInitialized
    );
    assert_eq!(current_user().unwrap_err(), FirebaseError::NotInitialized);
}
