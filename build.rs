//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use std::path::Path;

const COMMANDS: &[&str] = &[
    "run_python",
    "register_function",
    "call_function",
    "read_variable",
];

fn main() {
    let py_main = concat!(env!("PWD"), "/src-python/main.py");
    let py_main_path = Path::new(py_main);
    if !py_main_path.exists() {
        let parent = py_main_path.parent().unwrap_or(py_main_path);
        std::fs::create_dir_all(parent).unwrap_or_default();
        std::fs::write(py_main_path, "# auto loaded on starts").unwrap_or_default();
    }

    tauri_plugin::Builder::new(COMMANDS)
        // .global_api_script_path("./api-iife.js")
        .android_path("android")
        .ios_path("ios")
        .build();
}
