//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use serde::de::DeserializeOwned;
use tauri::{
    path::BaseDirectory,
    plugin::{PluginApi, PluginHandle},
    AppHandle, Manager, Runtime,
};

use crate::models::*;
use crate::py_lib;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_python);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<Python<R>> {
    #[cfg(target_os = "android")]
    let handle = api.register_android_plugin("com.plugin.python.application", "ExamplePlugin")?;
    #[cfg(target_os = "ios")]
    let handle = api.register_ios_plugin(init_plugin_python)?;
    let py_file_path = app
        .path()
        .resolve("src-python/main.py", BaseDirectory::Resource)?;
    let code = std::fs::read_to_string(py_file_path).expect("Error reading main.py");
    py_lib::init_python(code)?;
    Ok(Python(handle))
}

pub struct Python<R: Runtime>(PluginHandle<R>);
