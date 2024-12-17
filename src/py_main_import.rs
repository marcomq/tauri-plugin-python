//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python
use std::{env, path::PathBuf};

pub fn get_py_path() -> PathBuf {
    env::current_dir().unwrap().join("src-python")
}

pub fn read_at_startup<'a>() -> String {
    let py_file_path = get_py_path().join("main.py");
    std::fs::read_to_string(py_file_path).unwrap_or_default()
    // include_str!(concat!(env!("PWD"),  "/src-tauri/src-python/main.py"))
}
