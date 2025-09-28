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
use async_py::{self, PyRunner};
use lazy_static::lazy_static;

pub use error::{Error, Result};
use models::*;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Mutex},
};

#[cfg(desktop)]
use desktop::Python;
#[cfg(mobile)]
use mobile::Python;

lazy_static! {
    static ref INIT_BLOCKED: AtomicBool = false.into();
    static ref FUNCTION_MAP: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the python APIs.

#[async_trait::async_trait]
pub trait PythonExt<R: Runtime> {
    fn python(&self) -> &Python<R>;
    fn runner(&self) -> &PyRunner;
    async fn run_python(&self, payload: StringRequest) -> crate::Result<StringResponse>;
    async fn register_function(&self, payload: RegisterRequest) -> crate::Result<StringResponse>;
    async fn call_function(&self, payload: RunRequest) -> crate::Result<StringResponse>;
    async fn read_variable(&self, payload: StringRequest) -> crate::Result<StringResponse>;
}

#[async_trait::async_trait]
impl<R: Runtime, T: Manager<R> + Sync> crate::PythonExt<R> for T {
    fn python(&self) -> &Python<R> {
        self.state::<Python<R>>().inner()
    }
    fn runner(&self) -> &PyRunner {
        self.state::<PyRunner>().inner()
    }
    async fn run_python(&self, payload: StringRequest) -> crate::Result<StringResponse> {
        self.runner().run(&payload.value).await?;
        Ok(StringResponse { value: "Ok".into() })
    }

    async fn register_function(&self, payload: RegisterRequest) -> crate::Result<StringResponse> {
        if INIT_BLOCKED.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("Cannot register after function called".into());
        }
        FUNCTION_MAP
            .lock()
            .unwrap()
            .insert(payload.python_function_call.clone());

        let _tmp = self
            .runner()
            .read_variable(&payload.python_function_call)
            .await?;
        if let Some(num_args) = payload.number_of_args {
            let py_analyze_sig = format!(
                r#"
from inspect import signature
if len(signature({}).parameters) != {}:
    raise Exception("Function parameters don't match in 'registerFunction'")
"#,
                &payload.python_function_call, num_args
            );
            self.runner().run(&py_analyze_sig).await.unwrap_or_else(|_| {
                    panic!("Number of args doesn't match signature of {}.", payload.python_function_call)
                });
        };
        Ok(StringResponse { value: "Ok".into() })
    }

    async fn call_function(&self, payload: RunRequest) -> crate::Result<StringResponse> {
        INIT_BLOCKED.store(true, std::sync::atomic::Ordering::Relaxed);
        let function_name = payload.function_name;
        if FUNCTION_MAP.lock().unwrap().get(&function_name).is_none() {
            return Err(Error::String(format!(
                "Function {function_name} has not been registered yet"
            )));
        }
        let py_res = self
            .runner()
            .call_function(&function_name, payload.args)
            .await?;
        let value = match py_res.as_str() {
            Some(s) => s.to_string(),
            None => py_res.to_string(),
        };  
        Ok(StringResponse {
            value,
        })
    }

    async fn read_variable(&self, payload: StringRequest) -> crate::Result<StringResponse> {
        let py_res = self.runner().read_variable(&payload.value).await?;
        Ok(StringResponse {
            value: py_res.to_string(),
        })
    }
}

fn get_resource_dir<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .resolve("src-python", BaseDirectory::Resource)
        .unwrap_or_default()
}

fn get_src_python_dir() -> PathBuf {
    std::env::current_dir().unwrap().join("src-python")
}

/// Initializes the plugin with functions
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    init_and_register(vec![])
}

fn cleanup_path_for_python(path: &PathBuf) -> String {
    dunce::canonicalize(path)
        .unwrap()
        .to_string_lossy()
        .replace("\\", "/")
}

fn print_path_for_python(path: &PathBuf) -> String {
    #[cfg(not(target_os = "windows"))]
    {
        format!("\"{}\"", cleanup_path_for_python(path))
    }
    #[cfg(target_os = "windows")]
    {
        format!("r\"{}\"", cleanup_path_for_python(path))
    }
}

async fn init_python(runner: &PyRunner, dir: PathBuf) {
    let sys_pyth_dir = print_path_for_python(&dir);
    let path_import = format!(
        r#"import sys
sys.path = sys.path + [{}]
"#,
        sys_pyth_dir,
    );
    runner
        .run(&path_import)
        .await
        .expect("ERROR: Error setting python path");
    #[cfg(feature = "venv")]
    {
        let venv_dir = dir.join(".venv").join("lib");
        if Path::exists(venv_dir.as_path()) {
            runner
                .set_venv(venv_dir.as_path())
                .await
                .expect("ERROR: Error setting venv for python");
        }
    }
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
            let runner = PyRunner::new();
            app.manage(runner);

            let mut dir = get_resource_dir(app);
            let mut main_py = dir.join("main.py");
            if !main_py.exists() {
                println!(
                    "Warning: 'src-tauri/main.py' seems not to be registered in 'tauri.conf.json'"
                );
                dir = get_src_python_dir();
                main_py = dir.join("main.py");
            }
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async move {
                    let runner = app.state::<PyRunner>().inner();
                    init_python(runner, dir.to_path_buf()).await;
                    runner
                        .run_file(main_py.as_path())
                        .await
                        .expect("ERROR: Error running 'src-tauri/main.py'");
                    register_python_functions(
                        app,
                        python_functions.iter().map(|s| s.to_string()).collect(),
                    ).await;
                    let functions = runner
                        .read_variable("_tauri_plugin_functions")
                        .await
                        .unwrap_or_default();
                    if let Ok(python_functions) = serde_json::from_value(functions) {
                        register_python_functions(app, python_functions).await;
                    }
                });

            Ok(())
        })
        .build()
}

async fn register_python_functions<R: Runtime>(
    app: &AppHandle<R>,
    python_functions: Vec<String>,
) {
    for function_name in python_functions {
        app.register_function(RegisterRequest {
            python_function_call: function_name.clone(),
            number_of_args: None,
        })
        .await
        .unwrap();
    }
}

#[cfg(test)]
mod tests;