//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-python-plugin

pub fn read_at_compile_time<'a>() -> &'a str {
    // no actual error, file auto generated in build.rs, no idea how to ignore this
    // moved to separate file to not ignore other errors
    include_str!(concat!(env!("PWD"), "/", "src-tauri/src-python/main.py"))
}
