//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

#[cfg(feature = "pyo3")]
use pyo3::PyErr;
use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error: {0}")]
    String(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Error::String(error.into())
    }
}

#[cfg(not(feature = "pyo3"))]
impl From<rustpython_vm::PyRef<rustpython_vm::builtins::PyBaseException>> for Error {
    fn from(error: rustpython_vm::PyRef<rustpython_vm::builtins::PyBaseException>) -> Self {
        let msg = format!("{:?}", &error);
        println!("error: {}", &msg);
        if let Some(tb) = error.traceback() {
            println!("Traceback (most recent call last):");
            for trace in tb.iter() {
                println!(
                    "  File \"{}\", line {}, in {}",
                    trace.frame.code.source_path,
                    trace.lineno.to_usize(),
                    trace.frame.code.obj_name
                );
            }
        }
        Error::String(msg)
    }
}

#[cfg(feature = "pyo3")]
impl From<PyErr> for Error {
    fn from(error: PyErr) -> Self {
        let msg = error.to_string();
        println!("error: {}", &msg);
        Error::String(msg)
    }
}

impl From<tauri::Error> for Error {
    fn from(error: tauri::Error) -> Self {
        Error::String(error.to_string())
    }
}
