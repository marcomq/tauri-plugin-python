//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::PythonExt;
use crate::Result;

#[command]
pub(crate) async fn run_python<R: Runtime>(
    app: AppHandle<R>,
    payload: StringRequest,
) -> Result<StringResponse> {
    app.run_python(payload)
}
#[command]
pub(crate) async fn register_function<R: Runtime>(
    app: AppHandle<R>,
    payload: RegisterRequest,
) -> Result<StringResponse> {
    app.register_function(payload)
}
#[command]
pub(crate) async fn call_function<R: Runtime>(
    app: AppHandle<R>,
    payload: RunRequest,
) -> Result<StringResponse> {
    app.call_function(payload)
}
#[command]
pub(crate) async fn read_variable<R: Runtime>(
    app: AppHandle<R>,
    payload: StringRequest,
) -> Result<StringResponse> {
    app.read_variable(payload)
}
