//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StringRequest {
    pub value: String,
}

use serde_json::Value as JsMany;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub python_function_call: String,
    pub number_of_args: Option<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    pub function_name: String,
    pub args: Vec<JsMany>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StringResponse {
    pub value: String,
}
