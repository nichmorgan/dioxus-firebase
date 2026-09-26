//! Main integration test suite runner.

mod common;

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "ios")]
mod ios;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod unsupported;
