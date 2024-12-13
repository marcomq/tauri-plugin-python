use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_python);

/// Access to the python plugin APIs.
pub struct Python<R: Runtime>(PluginHandle<R>);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<Python<R>> {
    #[cfg(target_os = "android")]
    let handle = api.register_android_plugin("", "ExamplePlugin")?;
    #[cfg(target_os = "ios")]
    let handle = api.register_ios_plugin(init_plugin_python)?;
    py_lib::init_python()?;
    Ok(Python(handle))
}

impl<R: Runtime> Python<R> {
    pub fn run_python(&self, payload: StringRequest) -> crate::Result<StringResponse> {
        py_lib::run_python(payload)?;
        Ok(StringResponse { value: "Ok".into() })
    }
    pub fn register_function(&self, payload: RegisterRequest) -> crate::Result<StringResponse> {
        py_lib::register_function(payload)?;
        Ok(StringResponse { value: "Ok".into() })
    }
    pub fn call_function(&self, payload: RunRequest) -> crate::Result<StringResponse> {
        let py_res = py_lib::call_function(payload)?;
        Ok(StringResponse {
            value: py_res.to_string(),
        })
    }
    pub fn read_variable(&self, payload: StringRequest) -> crate::Result<StringResponse> {
        let py_res = py_lib::read_variable(payload)?;
        Ok(StringResponse { value: py_res })
    }
}
