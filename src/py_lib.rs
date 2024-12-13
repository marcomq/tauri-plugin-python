use std::{collections::HashMap, ffi::CString, sync::Mutex};

use lazy_static::lazy_static;
use pyo3::exceptions::PyBaseException;
use pyo3::types::{PyAnyMethods, PyDictMethods};
use pyo3::PyErr;
use pyo3::{marker, types::PyDict, Py, PyAny, PyResult};

use crate::models::*;
use crate::py_main_import;

lazy_static! {
    static ref FUNCTION_MAP: Mutex<HashMap<String, Py<PyAny>>> = Mutex::new(HashMap::new());
    static ref GLOBALS: Mutex<Py<PyDict>> =
        Mutex::new(marker::Python::with_gil(|py| { PyDict::new(py).into() }));
}
pub fn init_python() -> PyResult<()> {
    pyo3::prepare_freethreaded_python();
    let code = py_main_import::read_at_compile_time();
    let c_code = CString::new(code).expect("error loading python");
    marker::Python::with_gil(|py| -> PyResult<()> {
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
    let fn_name = payload.function_name;
    // TODO, check actual function signature
    marker::Python::with_gil(|py| -> PyResult<()> {
        let globals = GLOBALS.lock().unwrap().clone_ref(py).into_bound(py);
        let app = globals.get_item(&fn_name)?;
        if app.is_none() {
            return Err(pyo3::exceptions::PyException::new_err(format!(
                "{} not found",
                &fn_name
            )));
        }
        let app = app.unwrap();
        if !app.is_callable() {
            return Err(pyo3::exceptions::PyException::new_err(format!(
                "{} not a callable function",
                &fn_name
            )));
        }
        if let Some(num_args) = payload.number_of_args {
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
        FUNCTION_MAP.lock().unwrap().insert(fn_name, app.into());
        Ok(())
    })
}
pub fn call_function(payload: RunRequest) -> PyResult<Py<PyAny>> {
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
        match globals.get_item(&payload.value) {
            Ok(opt_res) => match opt_res {
                Some(res) => Ok(res.to_string()),
                _ => Err(pyo3::exceptions::PyException::new_err(format!(
                    "{} not set",
                    payload.value
                ))),
            },
            Err(err) => Err(err),
        }
    })
}
