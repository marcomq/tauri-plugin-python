//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use std::env;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::{collections::HashMap, ffi::CString, sync::Mutex};

use lazy_static::lazy_static;
use pyo3::exceptions::PyBaseException;
use pyo3::types::{PyAnyMethods, PyDictMethods, PyList, PyListMethods};
use pyo3::PyErr;
use pyo3::{marker, types::PyDict, Py, PyAny, PyResult};

use crate::models::*;

lazy_static! {
    static ref INIT_BLOCKED: AtomicBool = false.into();
    static ref FUNCTION_MAP: Mutex<HashMap<String, Py<PyAny>>> = Mutex::new(HashMap::new());
    static ref GLOBALS: Mutex<Py<PyDict>> =
        Mutex::new(marker::Python::with_gil(|py| { PyDict::new(py).into() }));
}

pub fn init_python(code: String) -> crate::Result<()> {
    let c_code = CString::new(code).expect("error creating cstring from code");
    marker::Python::with_gil(|py| -> PyResult<()> {
        let syspath = py
            .import("sys")?
            .getattr("path")?
            .downcast_into::<PyList>()?;
        syspath.insert(0, get_py_path().to_str())?;
        let globals = GLOBALS.lock().unwrap().clone_ref(py).into_bound(py);
        py.run(&c_code, Some(&globals), None)
    })
}

pub fn run_python(payload: StringRequest) -> PyResult<()> {
    marker::Python::with_gil(|py| {
        let globals = GLOBALS.lock().unwrap().clone_ref(py).into_bound(py);
        let code_c = CString::new(payload.value).expect("CString::new failed");
        py.run(&code_c, Some(&globals), None)
    })
}
pub fn register_function(payload: RegisterRequest) -> PyResult<()> {
    register_function_str(payload.python_function_call, payload.number_of_args)
}

pub fn register_function_str(fn_name: String, number_of_args: Option<u8>) -> PyResult<()> {
    // TODO, check actual function signature
    if INIT_BLOCKED.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(pyo3::exceptions::PyException::new_err(
            "Cannot register after function called",
        ));
    }
    marker::Python::with_gil(|py| -> PyResult<()> {
        let globals = GLOBALS.lock().unwrap().clone_ref(py).into_bound(py);

        let fn_dot_split: Vec<&str> = fn_name.split(".").collect();
        let app = globals.get_item(&fn_dot_split[0])?;
        if app.is_none() {
            return Err(pyo3::exceptions::PyException::new_err(format!(
                "{} not found",
                &fn_name
            )));
        }
        let app = if fn_dot_split.len() > 1 {
            app.unwrap().getattr(fn_dot_split.get(1).unwrap())?
        } else {
            app.unwrap()
        };
        if !app.is_callable() {
            return Err(pyo3::exceptions::PyException::new_err(format!(
                "{} not a callable function",
                &fn_name
            )));
        }
        if let Some(num_args) = number_of_args {
            let py_analyze_sig = format!(
                r#"
if True:
    from inspect import signature
    if len(signature({}).parameters) != {}:
        raise Exception("Function parameters don't match in 'registerFunction'")
    "#,
                fn_name, num_args
            );
            let code_c = CString::new(py_analyze_sig).expect("CString::new failed");
            py.run(&code_c, Some(&globals), None)
                .expect(&format!("Could not register '{}'. ", &fn_name));
        }
        // dbg!("{} was inserted", &fn_name);
        FUNCTION_MAP.lock().unwrap().insert(fn_name, app.into());
        Ok(())
    })
}
pub fn call_function(payload: RunRequest) -> PyResult<Py<PyAny>> {
    INIT_BLOCKED.store(true, std::sync::atomic::Ordering::Relaxed);
    marker::Python::with_gil(|py| -> PyResult<Py<PyAny>> {
        let arg = pyo3::types::PyTuple::new(py, payload.args)?;
        let map = FUNCTION_MAP
            .lock()
            .map_err(|msg| PyErr::new::<PyBaseException, _>(msg.to_string()))?;
        match map.get(&payload.function_name) {
            Some(app) => {
                // dbg!(&arg);
                let res = app.call1(py, arg);
                // dbg!(&res);
                res
            }
            _ => Err(pyo3::exceptions::PyException::new_err(format!(
                "{} not found",
                payload.function_name
            ))),
        }
    })
}

pub fn read_variable(payload: StringRequest) -> PyResult<String> {
    marker::Python::with_gil(|py| -> PyResult<String> {
        let globals = GLOBALS.lock().unwrap().clone_ref(py).into_bound(py);

        let var_dot_split: Vec<&str> = payload.value.split(".").collect();
        let var = globals.get_item(&var_dot_split[0])?;
        if let Some(var) = var {
            if var_dot_split.len() > 1 {
                Ok(var.getattr(var_dot_split.get(1).unwrap())?.to_string())
            } else {
                Ok(var.to_string())
            }
        } else {
            Err(pyo3::exceptions::PyException::new_err(format!(
                "{} not set",
                &payload.value
            )))
        }
    })
}
