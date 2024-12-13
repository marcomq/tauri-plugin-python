use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;
use crate::py_lib;

/// Access to the python plugin APIs.
pub struct Python<R: Runtime>(AppHandle<R>);

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Python<R>> {
    py_lib::init_python()?;
    Ok(Python(app.clone()))
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
