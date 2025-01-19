//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
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

pub use error::{Error, Result};
use models::*;

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
        let py_res = py_lib::call_function(payload)?;
        Ok(StringResponse {
            value: py_res.to_string(),
        })
    }
    fn read_variable(&self, payload: StringRequest) -> crate::Result<StringResponse> {
        let py_res = py_lib::read_variable(payload)?;
        Ok(StringResponse { value: py_res })
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>(python_functions: Vec<&'static str>) -> TauriPlugin<R> {
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
            for function_name in python_functions {
                py_lib::register_function_str(function_name.into(), None).unwrap();
            }
            Ok(())
        })
        .build()
}
