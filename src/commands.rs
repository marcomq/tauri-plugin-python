use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::PythonExt;
use crate::Result;

#[command]
pub(crate) async fn run_python<R: Runtime>(
    app: AppHandle<R>,
    payload: StringRequest,
) -> Result<StringResponse> {
    app.python().run_python(payload)
}
#[command]
pub(crate) async fn register_function<R: Runtime>(
    app: AppHandle<R>,
    payload: RegisterRequest,
) -> Result<StringResponse> {
    app.python().register_function(payload)
}
#[command]
pub(crate) async fn call_function<R: Runtime>(
    app: AppHandle<R>,
    payload: RunRequest,
) -> Result<StringResponse> {
    app.python().call_function(payload)
}
#[command]
pub(crate) async fn read_variable<R: Runtime>(
    app: AppHandle<R>,
    payload: StringRequest,
) -> Result<StringResponse> {
    app.python().read_variable(payload)
}
