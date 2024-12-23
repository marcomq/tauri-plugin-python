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
        .global_api_script_path("dist-js/index.iife.js")
        .android_path("android")
        .ios_path("ios")
        .build();
}
