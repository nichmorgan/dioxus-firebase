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

#[derive(Debug, Clone, Copy)]
enum Primitive {
    SignInWithEmail,
    CurrentUser,
    SignInWithGoogle,
    SignInWithApple,
    LinkWithGoogle,
    LinkWithApple,
}

fn call_primitive(which: Primitive) -> Result<(), FirebaseError> {
    match which {
        Primitive::SignInWithEmail => sign_in_with_email("a@b.c", "pass").map(|_| ()),
        Primitive::CurrentUser => current_user().map(|_| ()),
        Primitive::SignInWithGoogle => sign_in_with_google().map(|_| ()),
        Primitive::SignInWithApple => sign_in_with_apple().map(|_| ()),
        Primitive::LinkWithGoogle => link_with_google().map(|_| ()),
        Primitive::LinkWithApple => link_with_apple().map(|_| ()),
    }
}

#[rstest]
#[case(Primitive::SignInWithEmail)]
#[case(Primitive::CurrentUser)]
#[case(Primitive::SignInWithGoogle)]
#[case(Primitive::SignInWithApple)]
#[case(Primitive::LinkWithGoogle)]
#[case(Primitive::LinkWithApple)]
fn test_primitives_require_init_on_desktop(#[case] which: Primitive) {
    assert_eq!(
        call_primitive(which).unwrap_err(),
        FirebaseError::NotInitialized
    );
}
