//! Register the iOS Swift host package with Dioxus CLI via manganis.
//!
//! This emits `SwiftPackageMetadata` so `dx` 0.7+ compiles
//! [`ios/`](../../../ios/) (product `DioxusFirebaseAuthHost`) and embeds it as a
//! framework. Runtime calls stay in [`super::ios`] (hand ObjC) — the generated
//! type is unused.

#![allow(dead_code)]

#[manganis::ffi("ios")]
extern "Swift" {
    /// Present only so manganis emits plugin metadata for `dx`.
    type DioxusFirebaseAuthHost;
}
