const KEYS: [&str; 6] = [
    "FIREBASE_API_KEY",
    "FIREBASE_APP_ID",
    "FIREBASE_PROJECT_ID",
    "FIREBASE_GOOGLE_WEB_CLIENT_ID",
    "FIREBASE_USE_AUTH_EMULATOR",
    "ANDROID_APPLICATION_ID",
];

fn main() {
    println!("cargo:rerun-if-changed=.env");
    let file: Vec<(String, String)> = dotenvy::from_path_iter(".env")
        .map(|iter| iter.flatten().collect())
        .unwrap_or_default();
    for key in KEYS {
        println!("cargo:rerun-if-env-changed={key}");
        let non_empty = |value: &String| !value.trim().is_empty();
        let value = std::env::var(key)
            .ok()
            .filter(non_empty)
            .or_else(|| {
                file.iter()
                    .find(|(name, _)| name == key)
                    .map(|(_, value)| value.clone())
            })
            .filter(non_empty);
        if let Some(value) = value {
            println!("cargo:rustc-env={key}={value}");
        }
    }
}
