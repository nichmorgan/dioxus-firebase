// swift-tools-version:5.9

import PackageDescription

// Product name must match the `type` in `src/native/ios_plugin.rs`.
// `dx` rewrites `type: .static` to `.dynamic` so the ObjC class is visible after dlopen.
let package = Package(
    name: "DioxusFirebaseAuthHost",
    platforms: [
        .iOS(.v15),
    ],
    products: [
        .library(
            name: "DioxusFirebaseAuthHost",
            type: .static,
            targets: ["DioxusFirebaseAuthHost"]
        ),
    ],
    dependencies: [
        .package(url: "https://github.com/firebase/firebase-ios-sdk.git", exact: "11.15.0"),
    ],
    targets: [
        .target(
            name: "DioxusFirebaseAuthHost",
            dependencies: [
                .product(name: "FirebaseAuth", package: "firebase-ios-sdk"),
                .product(name: "FirebaseCore", package: "firebase-ios-sdk"),
            ],
            path: ".",
            sources: ["DioxusFirebaseAuthHost.swift"]
        ),
    ]
)
