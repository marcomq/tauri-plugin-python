//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

// python3 -m venv .venv
// .venv/bin/activate
// pip install pyoxidizer

const COMMANDS: &[&str] = &[
    "run_python",
    "register_function",
    "call_function",
    "read_variable",
];

#[cfg(feature = "pyembed")]
fn init_pyembed() {
    use std::path::Path;
    let pyoxidizer_exe = if let Ok(path) = std::env::var("PYOXIDIZER_EXE") {
        path
    } else {
        "pyoxidizer".to_string()
    };
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let pyembed_dir = format!("{}/../../../../pyembed", out_dir);
    let args = vec!["generate-python-embedding-artifacts", &pyembed_dir];
    // Setting PYO3_CONFIG_FILE here wouldn't set it while compiling pyo3
    // Therefore, we need to set it manually:
    // `PYO3_CONFIG_FILE=${PWD}/src-tauri/target/pyembed/pyo3-build-config-file.txt npm run tauri dev`
    // println!("cargo::rustc-env=PYO3_CONFIG_FILE={}/{}", pyembed_dir, "pyo3-build-config-file.txt");
    match std::process::Command::new(pyoxidizer_exe)
        .args(args)
        .status()
    {
        Ok(status) => {
            if !status.success() {
                panic!("`pyoxidizer run-build-script` failed");
            }
            let src_tauri_dir = format!("{}/../../../../../../src-tauri", out_dir);
            let cargo_config = Path::new(&src_tauri_dir).join(".cargo").join("config.toml");
            dbg!(&cargo_config);
            if !cargo_config.exists() {
                let content = r#"
[env]
PYO3_CONFIG_FILE = { value = "target/pyembed/pyo3-build-config-file.txt", relative = true }"#;
                let _ignore = std::fs::create_dir(Path::new(&src_tauri_dir).join(".cargo"));
                std::fs::write(cargo_config, content).unwrap();
            }
        }
        Err(e) => panic!(
            "`pyoxidizer run-build-script` failed, please install pyoxidizer first: {}/n
        cargo install pyoxidizer",
            e.to_string()
        ),
    }
}

#[cfg(not(feature = "pyembed"))]
fn init_pyembed() {}

fn main() {
    init_pyembed();
    tauri_plugin::Builder::new(COMMANDS)
        .global_api_script_path("./dist-js/index.iife.js")
        .android_path("android")
        .ios_path("ios")
        .build();
}
