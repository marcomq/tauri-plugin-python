//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use tauri::{
    AppHandle, Manager, Runtime,
    path::BaseDirectory,
    plugin::{Builder, TauriPlugin},
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
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Mutex, atomic::AtomicBool},
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
/// Errors include Python tracebacks if [`tracebacks_enabled`].
fn build_runner() -> PyRunner {
    let runner = PyRunner::new().with_tracebacks(tracebacks_enabled(
        std::env::var(TRACEBACK_ENV).ok().as_deref(),
    ));
    match call_timeout(
        std::env::var("TAURI_PLUGIN_PYTHON_TIMEOUT_SECS")
            .ok()
            .as_deref(),
    ) {
        Some(timeout) => runner.with_timeout(timeout),
        None => runner,
    }
}

const TRACEBACK_ENV: &str = "TAURI_PLUGIN_PYTHON_TRACEBACK";

/// Tracebacks are on in debug builds, and in release builds if the
/// `TAURI_PLUGIN_PYTHON_TRACEBACK` env var is `1` or `true`.
fn tracebacks_enabled(env_value: Option<&str>) -> bool {
    cfg!(debug_assertions)
        || env_value.is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true"))
}

/// Resolves the per-call timeout from the raw env var value; `None` disables it.
fn call_timeout(env_value: Option<&str>) -> Option<Duration> {
    match env_value.and_then(|v| v.trim().parse::<u64>().ok()) {
        Some(0) => None,
        Some(secs) => Some(Duration::from_secs(secs)),
        None => Some(Duration::from_secs(DEFAULT_TIMEOUT_SECS)),
    }
}

#[cfg(desktop)]
use desktop::Python;
#[cfg(mobile)]
use mobile::Python;

#[derive(Default)]
struct PluginState {
    init_blocked: AtomicBool,
    /// Registered functions, mapped to whether they are `async def` coroutine functions.
    function_map: Mutex<HashMap<String, bool>>,
}

/// Prepends human-readable context to a failing Python operation and, in debug
/// builds (`tauri dev`), prints the full detail - including the Python traceback
/// carried in the underlying error - to stderr so it is visible in the dev
/// console. The full message, traceback included, also reaches the frontend.
/// Release builds log nothing and their errors carry no traceback, unless
/// `TAURI_PLUGIN_PYTHON_TRACEBACK` enables both (see [`tracebacks_enabled`]).
fn py_context<T, E: Into<Error>>(
    result: std::result::Result<T, E>,
    context: impl FnOnce() -> String,
) -> crate::Result<T> {
    result.map_err(|err| {
        let msg = format!("{}: {}", context(), err.into());
        if tracebacks_enabled(std::env::var(TRACEBACK_ENV).ok().as_deref()) {
            eprintln!("[tauri-plugin-python] {msg}");
        }
        Error::String(msg)
    })
}

// `async_trait` expands to `#[must_use]` on functions returning a boxed future.
/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the python APIs.
#[allow(clippy::double_must_use)]
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
            // itself can't run (e.g. builtins without a signature), registration
            // proceeds without validation.
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
                payload.python_function_call, num_args
            );
            self.runner().run(&py_analyze_sig).await.map_err(|_| {
                Error::String(format!(
                    "Function parameters don't match signature of {}.",
                    payload.python_function_call
                ))
            })?;
        };
        let is_async = py_context(
            is_coroutine_function(self.runner(), &payload.python_function_call).await,
            || format!("Cannot register '{}'", payload.python_function_call),
        )?;
        state
            .function_map
            .lock()
            .unwrap()
            .insert(payload.python_function_call.clone(), is_async);
        Ok(StringResponse { value: "Ok".into() })
    }

    async fn call_function(&self, payload: RunRequest) -> crate::Result<StringResponse> {
        let state = self.state::<PluginState>().inner();
        state
            .init_blocked
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let function_name = payload.function_name;
        let is_async = state
            .function_map
            .lock()
            .unwrap()
            .get(&function_name)
            .copied();
        let Some(is_async) = is_async else {
            return Err(Error::String(format!(
                "Function {function_name} has not been registered yet"
            )));
        };
        let call = if is_async {
            self.runner()
                .call_async_function(&function_name, payload.args)
                .await
        } else {
            self.runner()
                .call_function(&function_name, payload.args)
                .await
        };
        let py_res = py_context(call, || {
            format!("Error calling Python function '{function_name}'")
        })?;
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

/// `async def` functions are awaited via `call_async_function` instead of
/// returning an un-awaited coroutine. Falls back to the code flags when
/// `inspect` is unavailable.
async fn is_coroutine_function(
    runner: &PyRunner,
    name: &str,
) -> std::result::Result<bool, async_py::PyRunnerError> {
    let check = format!(
        r#"
try:
    from inspect import iscoroutinefunction as _tauri_iscoro
except Exception:
    def _tauri_iscoro(f):
        return bool(getattr(getattr(f, "__code__", None), "co_flags", 0) & 0x80)
_tauri_is_async = bool(_tauri_iscoro({name}))
"#
    );
    runner.run(&check).await?;
    Ok(runner.read_variable("_tauri_is_async").await? == serde_json::Value::Bool(true))
}

fn get_resource_dir<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .resolve("src-python", BaseDirectory::Resource)
        .unwrap_or_default()
}

/// The Python entry point of the app inside a `src-python` directory.
#[derive(Debug, PartialEq)]
enum PythonEntry {
    /// A `main.py` source file, executed with `run_file`.
    File(PathBuf),
    /// A compiled module (e.g. `nuitka --module main.py`) in `dir`, imported via `run_module`.
    Module { name: String, dir: PathBuf },
}

impl std::fmt::Display for PythonEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PythonEntry::File(path) => write!(f, "'{}'", path.display()),
            PythonEntry::Module { name, dir } => {
                write!(f, "compiled module '{name}' from '{}'", dir.display())
            }
        }
    }
}

const ENTRY_MODULE: &str = "main";

/// Prefers `main.py`; otherwise falls back to a compiled `main` module
/// (`main.*.so`, `main.*.pyd` or `main.pyc`) as produced by Nuitka or Cython.
fn find_entry(dir: &Path) -> Option<PythonEntry> {
    let main_py = dir.join("main.py");
    if main_py.is_file() {
        return Some(PythonEntry::File(main_py));
    }
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .any(|e| e.path().is_file() && is_compiled_entry(&e.file_name().to_string_lossy()))
        .then(|| PythonEntry::Module {
            name: ENTRY_MODULE.into(),
            dir: dir.to_path_buf(),
        })
}

fn is_compiled_entry(file_name: &str) -> bool {
    let prefix = format!("{ENTRY_MODULE}.");
    file_name.starts_with(&prefix)
        && [".so", ".pyd", ".pyc"]
            .iter()
            .any(|ext| file_name.ends_with(ext))
}

async fn run_entry(runner: &PyRunner, entry: &PythonEntry) -> crate::Result<()> {
    match entry {
        PythonEntry::File(path) => runner.run_file(path).await?,
        PythonEntry::Module { name, dir } => {
            runner.run(&load_module_from(name, dir)).await?;
            runner.run_module(name).await?
        }
    }
    Ok(())
}

/// Loads `name` from `dir` into `sys.modules`, so `run_module` can't pick up a
/// same-named module from elsewhere on `sys.path` or a stale `sys.modules` entry.
fn load_module_from(name: &str, dir: &Path) -> String {
    format!(
        r#"
def _tauri_load(name, path):
    import importlib.machinery, importlib.util, sys
    spec = importlib.machinery.PathFinder.find_spec(name, [path])
    if spec is None:
        raise ModuleNotFoundError(f"No module named '{{name}}' in {{path}}", name=name)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    try:
        spec.loader.exec_module(module)
    except BaseException:
        del sys.modules[name]
        raise
try:
    _tauri_load({name:?}, {path})
finally:
    del _tauri_load
"#,
        path = print_path_for_python(&dir.to_path_buf())
    )
}

fn get_src_python_dir() -> PathBuf {
    std::env::current_dir().unwrap().join("src-python")
}

/// Uses a bundled standalone Python (`pyembed/python`) if the app ships one.
#[cfg(all(feature = "pyo3", not(feature = "rustpython")))]
fn init_embedded_python<R: Runtime>(app: &AppHandle<R>) -> crate::Result<()> {
    const EMBEDDED_PYTHON_DIR: &str = "pyembed/python";
    let candidates = [
        app.path()
            .resolve(EMBEDDED_PYTHON_DIR, BaseDirectory::Resource)
            .ok(),
        // Release builds must not pick up a Python from wherever they are started.
        std::env::current_dir()
            .ok()
            .filter(|_| cfg!(debug_assertions))
            .map(|dir| dir.join(EMBEDDED_PYTHON_DIR)),
    ];
    let Some(home) = candidates
        .into_iter()
        .flatten()
        .find(|dir| async_py::is_python_home(dir))
    else {
        return Ok(());
    };
    if !async_py::init_python_home(&home)? {
        eprintln!(
            "[tauri-plugin-python] Warning: Python was initialized before the plugin; bundled Python at '{}' is not used",
            home.display()
        );
    }
    Ok(())
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
        let venv_dir = dir.join(".venv");
        if venv_dir.is_dir()
            && let Err(e) = runner.set_venv(&venv_dir).await
        {
            eprintln!(
                "[tauri-plugin-python] Warning: '{}' not loaded, it doesn't match this Python: {e}",
                venv_dir.display()
            );
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
            #[cfg(all(feature = "pyo3", not(feature = "rustpython")))]
            init_embedded_python(app)?;
            let runner = build_runner();
            app.manage(runner);
            app.manage(PluginState::default());

            let mut dir = get_resource_dir(app);
            let mut entry = find_entry(&dir);
            if entry.is_none() {
                eprintln!(
                    "[tauri-plugin-python] Warning: no main.py or compiled 'main' module in '{}'; is 'src-python/' listed in bundle.resources of tauri.conf.json?",
                    dir.display()
                );
                // Release builds must not run Python code from wherever they are started.
                if cfg!(debug_assertions) {
                    dir = get_src_python_dir();
                    entry = find_entry(&dir);
                }
            }
            let entry = entry.unwrap_or_else(|| PythonEntry::File(dir.join("main.py")));
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async move {
                    let runner = app.state::<PyRunner>().inner();
                    init_python(runner, dir.to_path_buf()).await;
                    if let Err(err) = run_entry(runner, &entry).await {
                        startup_failed(&format!("Failed to load {entry}"), err);
                    }
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
        if let Err(err) = app
            .register_function(RegisterRequest {
                python_function_call: function_name.clone(),
                number_of_args: None,
            })
            .await
        {
            startup_failed("Failed to register Python functions", err);
        }
    }
}

/// Aborts plugin initialization with a readable message instead of a `Debug`-formatted error.
fn startup_failed(context: &str, err: impl std::fmt::Display) -> ! {
    let mut msg = format!("[tauri-plugin-python] {context}: {err}");
    if msg.contains("bad magic number") {
        msg.push_str(
            "\nHint: the compiled module was built for a different Python version than the one the app runs",
        );
    }
    panic!("{msg}");
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod unit_tests;
