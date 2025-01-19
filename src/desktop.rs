//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use serde::de::DeserializeOwned;
use tauri::{ plugin::PluginApi, AppHandle, Runtime};

use crate::py_lib;

/// Access to the python plugin APIs.
pub struct Python<R: Runtime>(AppHandle<R>);

fn read_main_py<'a>() -> String {
    let py_file_path = std::env::current_dir().unwrap().join("src-python").join("main.py");
    std::fs::read_to_string(py_file_path).unwrap_or_default()
    // include_str!(concat!(env!("PWD"),  "/src-tauri/src-python/main.py"))
}

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Python<R>> {
    py_lib::init_python(read_main_py()).unwrap();
    Ok(Python(app.clone()))
}
