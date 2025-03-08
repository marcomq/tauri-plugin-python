//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

#[cfg(feature = "pyo3")]
use pyo3::{prelude::*, PyErr};
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
                let file = trace.frame.code.source_path.as_str();
                let original_line = trace.lineno.to_usize();
                let line = if file == "main.py" {
                    original_line - 2 // sys.path import has 2 additional lines
                } else {
                    original_line
                };
                println!(
                    "  File \"{file}\", line {line}, in {}",
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
        let error_msg = match pyo3::Python::with_gil(|py| -> Result<Vec<String>> {
            let traceback_module = py.import("traceback")?;
            let traceback_object = error
                .traceback(py)
                .ok_or(pyo3::exceptions::PyWarning::new_err("No traceback found."))?;
            let extract_traceback = traceback_module.getattr("extract_tb")?;

            // Get the formatted traceback lines
            let result = extract_traceback.call1((traceback_object,)).and_then(|r| {
                match r.extract::<Vec<PyObject>>() {
                    Ok(v) => {
                        let mut formatted_lines = Vec::new();
                        for arg in v.iter() {
                            let frame = arg.bind(py);

                            // Extract filename
                            let filename = match frame.getattr("filename") {
                                Ok(f) => match f.extract::<String>() {
                                    Ok(s) if s == "<string>".to_string() => {
                                        // Special handling for <string>
                                        frame.setattr("filename", "main.py")?;
                                        let lineno = frame.getattr("lineno")?.extract::<usize>()?;
                                        frame.setattr("lineno", lineno - 2)?;
                                        "main.py".to_string()
                                    }
                                    Ok(s) => s,
                                    Err(_) => "<unknown>".to_string(),
                                },
                                Err(_) => "<unknown>".to_string(),
                            };

                            // Extract line number
                            let lineno = match frame.getattr("lineno") {
                                Ok(l) => match l.extract::<usize>() {
                                    Ok(n) => n,
                                    Err(_) => 0,
                                },
                                Err(_) => 0,
                            };

                            // Extract function name
                            let name = match frame.getattr("name") {
                                Ok(n) => match n.extract::<String>() {
                                    Ok(s) => s,
                                    Err(_) => "<unknown>".to_string(),
                                },
                                Err(_) => "<unknown>".to_string(),
                            };

                            // Extract line content (if available)
                            let line = match frame.getattr("line") {
                                Ok(l) => match l.extract::<Option<String>>() {
                                    Ok(Some(s)) => format!("\t{}", s),
                                    _ => "".to_string(),
                                },
                                Err(_) => "".to_string(),
                            };

                            // Format the line like requested
                            let formatted_line = format!(
                                "File \"{}\", line {}, in {}\n{}",
                                filename, lineno, name, line
                            );

                            formatted_lines.push(formatted_line);
                        }

                        Ok(formatted_lines)
                    }
                    Err(_) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "Failed to extract traceback",
                    )),
                }
            })?;

            // Add traceback header
            let mut full_traceback = vec!["Traceback (most recent call last):".to_string()];
            full_traceback.extend(result);

            // Add error type and message
            full_traceback.push(error.to_string());

            Ok(full_traceback)
        }) {
            Ok(formatted) => formatted.join("\n"),
            Err(_) => error.to_string(), // Fall back to simple error message
        };

        Error::String(error_msg)
    }
}

impl From<tauri::Error> for Error {
    fn from(error: tauri::Error) -> Self {
        Error::String(error.to_string())
    }
}
