//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-python-plugin

use pyo3::PyErr;
use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
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

// probably not the best solution - please optimize :)
impl From<&str> for Error {
    fn from(error: &str) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, error).into()
    }
}

impl From<Error> for PyErr {
    fn from(error: Error) -> Self {
        pyo3::exceptions::PyValueError::new_err(error.to_string())
    }
}

impl From<PyErr> for Error {
    fn from(error: PyErr) -> Self {
        let msg = error.to_string();
        println!("{}", &msg);
        std::io::Error::new(std::io::ErrorKind::Other, msg).into()
    }
}

impl From<tauri::Error> for Error {
    fn from(error: tauri::Error) -> Self {
        error.into()
    }
}
