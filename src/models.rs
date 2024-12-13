use pyo3::IntoPyObject;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StringRequest {
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, IntoPyObject)]
#[serde(untagged)]
pub enum JsMany {
    Bool(bool),
    Number(u64),
    Float(f64),
    String(String),
    StringVec(Vec<String>),
    FloatVec(Vec<f64>),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub function_name: String,
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
