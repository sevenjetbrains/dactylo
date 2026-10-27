fn main() {
    // Sous Windows, le manifeste (Common Controls v6) est lié à tous les binaires,
    // y compris ceux de `cargo test` : sans lui, ils échouent avec STATUS_ENTRYPOINT_NOT_FOUND.
    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("tauri-build");

    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows-msvc") {
        let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
}
