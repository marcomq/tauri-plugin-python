//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

const COMMANDS: &[&str] = &[
    "run_python",
    "register_function",
    "call_function",
    "read_variable",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .global_api_script_path("./dist-js/index.iife.js")
        .android_path("android")
        .ios_path("ios")
        .build();

    // Windows: embed a Common-Controls v6 application manifest into `cargo test`
    // binaries. The tauri/wry/tao chain (linked here via the `tauri` test
    // dev-dependency) hard-imports comctl32 v6 symbols such as
    // `TaskDialogIndirect`; without the v6 manifest the loader binds against the
    // legacy v5 comctl32.dll, which doesn't export them, and the test exe aborts
    // at startup with STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139) before any test
    // runs. Normal tauri app builds embed their own manifest, but test binaries
    // don't - hence this manifest. The unit tests live in the `lib` target
    // (not an integration-test target), so `rustc-link-arg-tests` doesn't apply
    // and errors with "does not have a test target"; the general
    // `rustc-link-arg` covers the lib unit-test harness (and only this crate's
    // own linked artifacts, not dependents).
    // See tauri-apps/tauri #13419, #13954, #11028 and discussion #11179.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        let manifest =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-app-manifest.xml");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        println!("cargo:rerun-if-changed=windows-app-manifest.xml");
    }
}
