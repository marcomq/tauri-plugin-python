//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use tauri::{
    path::BaseDirectory,
    plugin::{Builder, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;
#[cfg(not(feature = "pyo3"))]
mod py_lib;
#[cfg(feature = "pyo3")]
mod py_lib_pyo3;
#[cfg(feature = "pyo3")]
use py_lib_pyo3 as py_lib;

pub use error::{Error, Result};
use models::*;
use std::path::PathBuf;

#[cfg(desktop)]
use desktop::Python;
#[cfg(mobile)]
use mobile::Python;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the python APIs.
pub trait PythonExt<R: Runtime> {
    fn python(&self) -> &Python<R>;
    fn run_python(&self, payload: StringRequest) -> crate::Result<StringResponse>;
    fn register_function(&self, payload: RegisterRequest) -> crate::Result<StringResponse>;
    fn call_function(&self, payload: RunRequest) -> crate::Result<StringResponse>;
    fn read_variable(&self, payload: StringRequest) -> crate::Result<StringResponse>;
}

impl<R: Runtime, T: Manager<R>> crate::PythonExt<R> for T {
    fn python(&self) -> &Python<R> {
        self.state::<Python<R>>().inner()
    }
    fn run_python(&self, payload: StringRequest) -> crate::Result<StringResponse> {
        py_lib::run_python(payload)?;
        Ok(StringResponse { value: "Ok".into() })
    }
    fn register_function(&self, payload: RegisterRequest) -> crate::Result<StringResponse> {
        py_lib::register_function(payload)?;
        Ok(StringResponse { value: "Ok".into() })
    }
    fn call_function(&self, payload: RunRequest) -> crate::Result<StringResponse> {
        let py_res: String = py_lib::call_function(payload)?;
        Ok(StringResponse { value: py_res })
    }
    fn read_variable(&self, payload: StringRequest) -> crate::Result<StringResponse> {
        let py_res = py_lib::read_variable(payload)?;
        Ok(StringResponse { value: py_res })
    }
}

fn get_resource_dir<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .resolve("src-python", BaseDirectory::Resource)
        .unwrap_or_default()
}

fn get_current_dir() -> PathBuf {
    std::env::current_dir().unwrap().join("src-python")
}

/// Initializes the plugin with functions
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    init_and_register(vec![])
}

fn init_python(code: String, dir: PathBuf) {
    let path_import_and_code = format!(
        r#"
import sys
sys.path.append('{}')
{}
"#,
        dir.to_str().unwrap(),
        code
    );
    py_lib::run_python(StringRequest {
        value: path_import_and_code,
    })
    .unwrap_or_else(|e| panic!("Error '{e}' initializing main.py."));
}

/// Initializes the plugin.
pub fn init_and_register<R: Runtime>(python_functions: Vec<&'static str>) -> TauriPlugin<R> {
    Builder::new("python")
        .invoke_handler(tauri::generate_handler![
            commands::run_python,
            commands::register_function,
            commands::call_function,
            commands::read_variable
        ])
        .setup(|app, api| {
            #[cfg(mobile)]
            let python = mobile::init(app, api)?;
            #[cfg(desktop)]
            let python = desktop::init(app, api)?;
            app.manage(python);

            let mut dir = get_resource_dir(app);
            let mut code = std::fs::read_to_string(dir.join("main.py")).unwrap_or_default();
            if code.is_empty() {
                println!(
                    "Warning: 'src-tauri/main.py' seems not to be registered in 'tauri.conf.json'"
                );
                dir = get_current_dir();
                code = std::fs::read_to_string(dir.join("main.py")).unwrap_or_default();
            }
            if code.is_empty() {
                println!("ERROR: Error reading 'src-tauri/main.py'");
            }
            init_python(code, dir);
            for function_name in python_functions {
                py_lib::register_function_str(function_name.into(), None).unwrap();
            }
            let functions = py_lib::read_variable(StringRequest {
                value: "_tauri_plugin_functions".into(),
            })
            .unwrap_or_default()
            .replace("'", "\""); // python arrays are serialized usings ' instead of "

            if let Ok(python_functions) = serde_json::from_str::<Vec<String>>(&functions) {
                for function_name in python_functions {
                    py_lib::register_function_str(function_name, None).unwrap();
                }
            }
            Ok(())
        })
        .build()
}
