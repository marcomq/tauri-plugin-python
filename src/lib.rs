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

pub use error::{Error, Result};
use models::*;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{atomic::AtomicBool, Mutex},
    time::Duration,
};

/// Default per-call timeout applied to the Python worker so a single wedged call
/// (e.g. a blocking `print()` on a hidden-console Windows build, or a network
/// call whose own timeout never fires) cannot hang every later call forever.
/// Generous on purpose so it won't interfere with legitimately long work.
/// Override via the `TAURI_PLUGIN_PYTHON_TIMEOUT_SECS` env var (`0` disables it).
const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Python executed once at startup, before `main.py`, to make stdio safe.
///
/// On a Windows release build the console is hidden (`windows_subsystem =
/// "windows"`), so the standard handles are missing/invalid and a bare
/// `print()` can raise or abort the whole process (see issues #4/#15/#17). This
/// wraps `sys.stdout`/`sys.stderr` so writes can never crash the app. It cannot
/// un-block a write that hangs on a full, unread pipe - the call timeout is the
/// backstop for that - but it removes the common crash-on-stdio failure mode.
const PY_STDIO_GUARD: &str = r#"import sys

class _TauriSafeStream:
    def __init__(self, real):
        self._real = real
    def write(self, data):
        try:
            if self._real is not None:
                return self._real.write(data)
        except Exception:
            pass
        return 0
    def flush(self):
        try:
            if self._real is not None:
                self._real.flush()
        except Exception:
            pass
    def isatty(self):
        try:
            return bool(self._real is not None and self._real.isatty())
        except Exception:
            return False
    def __getattr__(self, name):
        return getattr(self._real, name)

sys.stdout = _TauriSafeStream(getattr(sys, "stdout", None))
sys.stderr = _TauriSafeStream(getattr(sys, "stderr", None))
"#;

/// Builds the shared [`PyRunner`], applying the default per-call timeout unless
/// the `TAURI_PLUGIN_PYTHON_TIMEOUT_SECS` env var overrides it (`0` = no timeout).
fn build_runner() -> PyRunner {
    let runner = PyRunner::new();
    match std::env::var("TAURI_PLUGIN_PYTHON_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
    {
        Some(0) => runner,
        Some(secs) => runner.with_timeout(Duration::from_secs(secs)),
        None => runner.with_timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS)),
    }
}

#[cfg(desktop)]
use desktop::Python;
#[cfg(mobile)]
use mobile::Python;

#[derive(Default)]
struct PluginState {
    init_blocked: AtomicBool,
    function_map: Mutex<HashSet<String>>,
}

/// Prepends human-readable context to a failing Python operation and, in debug
/// builds (`tauri dev`), prints the full detail - including the Python traceback
/// carried in the underlying error - to stderr so it is visible in the dev
/// console. The original error message is preserved in the returned error, so it
/// also still reaches the frontend. In release builds nothing is logged.
fn py_context<T, E: Into<Error>>(
    result: std::result::Result<T, E>,
    context: impl FnOnce() -> String,
) -> crate::Result<T> {
    result.map_err(|err| {
        let msg = format!("{}: {}", context(), err.into());
        #[cfg(debug_assertions)]
        eprintln!("[tauri-plugin-python] {msg}");
        Error::String(msg)
    })
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
        py_context(self.runner().run(&payload.value).await, || {
            "Error running Python code (runPython)".into()
        })?;
        Ok(StringResponse { value: "Ok".into() })
    }

    async fn register_function(&self, payload: RegisterRequest) -> crate::Result<StringResponse> {
        let state = self.state::<PluginState>().inner();
        if state
            .init_blocked
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return Err("Cannot register after function called".into());
        }
        let _tmp = py_context(
            self.runner()
                .read_variable(&payload.python_function_call)
                .await,
            || {
                format!(
                    "Cannot register '{}': not found in Python (is it defined/imported in main.py?)",
                    payload.python_function_call
                )
            },
        )?;
        if let Some(num_args) = payload.number_of_args {
            // Validate the argument count via `inspect.signature`, but only
            // *reject* the registration on an actual mismatch. If the check
            // itself can't run - e.g. the RustPython backend can't import
            // `inspect` - the import failure is swallowed in Python so the call
            // succeeds and registration proceeds without validation, rather
            // than failing on an unrelated error.
            let py_analyze_sig = format!(
                r#"
try:
    from inspect import signature
    _tauri_param_count = len(signature({0}).parameters)
except Exception:
    _tauri_param_count = None
if _tauri_param_count is not None and _tauri_param_count != {1}:
    raise Exception("Function parameters don't match in 'registerFunction'")
"#,
                &payload.python_function_call, num_args
            );
            self.runner().run(&py_analyze_sig).await.map_err(|_| {
                Error::String(format!(
                    "Function parameters don't match signature of {}.",
                    payload.python_function_call
                ))
            })?;
        };
        state
            .function_map
            .lock()
            .unwrap()
            .insert(payload.python_function_call.clone());
        Ok(StringResponse { value: "Ok".into() })
    }

    async fn call_function(&self, payload: RunRequest) -> crate::Result<StringResponse> {
        let state = self.state::<PluginState>().inner();
        state
            .init_blocked
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let function_name = payload.function_name;
        if state
            .function_map
            .lock()
            .unwrap()
            .get(&function_name)
            .is_none()
        {
            return Err(Error::String(format!(
                "Function {function_name} has not been registered yet"
            )));
        }
        let py_res = py_context(
            self.runner()
                .call_function(&function_name, payload.args)
                .await,
            || format!("Error calling Python function '{function_name}'"),
        )?;
        let value = match py_res.as_str() {
            Some(s) => s.to_string(),
            None => py_res.to_string(),
        };
        Ok(StringResponse { value })
    }

    async fn read_variable(&self, payload: StringRequest) -> crate::Result<StringResponse> {
        let py_res = py_context(self.runner().read_variable(&payload.value).await, || {
            format!("Error reading Python variable '{}'", payload.value)
        })?;
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
    // Make stdio safe before anything else (incl. main.py) runs - see PY_STDIO_GUARD.
    runner
        .run(PY_STDIO_GUARD)
        .await
        .expect("ERROR: Error initializing python stdio");
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
        if venv_dir.exists() {
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
            let runner = build_runner();
            app.manage(runner);
            app.manage(PluginState::default());

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
                    )
                    .await;
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

async fn register_python_functions<R: Runtime>(app: &AppHandle<R>, python_functions: Vec<String>) {
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
