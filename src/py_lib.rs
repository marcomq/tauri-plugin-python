//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use std::env;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::{collections::HashSet, sync::Mutex};

use rustpython_vm::{py_serde, PyResult};

use lazy_static::lazy_static;

use crate::models::*;

fn create_globals() -> rustpython_vm::scope::Scope {
    rustpython_vm::Interpreter::without_stdlib(Default::default())
        .enter(|vm| vm.new_scope_with_builtins())
}

lazy_static! {
    static ref INIT_BLOCKED: AtomicBool = false.into();
    static ref FUNCTION_MAP: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    static ref GLOBALS: rustpython_vm::scope::Scope = create_globals();
}

fn get_py_path() -> PathBuf {
    env::current_dir().unwrap().join("src-python")
}

fn read_main_py<'a>() -> String {
    let py_file_path = get_py_path().join("main.py");
    std::fs::read_to_string(py_file_path).unwrap()
    // include_str!(concat!(env!("PWD"),  "/src-tauri/src-python/main.py"))
}

pub fn init_python() -> PyResult<()> {
    let code = read_main_py();
    rustpython_vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
        let code_obj = vm
            .compile(
                &code,
                rustpython_vm::compiler::Mode::Exec,
                "<embedded>".to_owned(),
            )
            .map_err(|err| vm.new_syntax_error(&err, Some(&code)))?;
        vm.run_code_obj(code_obj, GLOBALS.clone())
    })?;
    Ok(())
}

pub fn run_python(payload: StringRequest) -> PyResult<()> {
    rustpython_vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
        let code_obj = vm
            .compile(
                &payload.value,
                rustpython_vm::compiler::Mode::Exec,
                "<embedded>".to_owned(),
            )
            .map_err(|err| vm.new_syntax_error(&err, Some(&payload.value)))?;
        vm.run_code_obj(code_obj, GLOBALS.clone())
    })?;
    Ok(())
}
pub fn register_function(payload: RegisterRequest) -> PyResult<()> {
    register_function_str(payload.python_function_call, payload.number_of_args)
}

pub fn register_function_str(fn_name: String, number_of_args: Option<u8>) -> PyResult<()> {
    rustpython_vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
        GLOBALS.globals.get_item(&fn_name, vm).unwrap();
        FUNCTION_MAP.lock().unwrap().insert(fn_name);
        Ok(())
    })
}
pub fn call_function(payload: RunRequest) -> PyResult<String> {
    // TODO,
    rustpython_vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
        let posargs: Vec<_> = payload
            .args
            .into_iter()
            .map(|x| py_serde::deserialize(vm, x).unwrap())
            .collect();
        let res = GLOBALS
            .globals
            .get_item(&payload.function_name, vm)?
            .call(posargs, vm)?
            .str(vm)?
            .to_string();
        Ok(res)
    })
}

pub fn read_variable(payload: StringRequest) -> PyResult<String> {
    rustpython_vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
        let res = GLOBALS
            .globals
            .get_item(&payload.value, vm)
            .unwrap()
            .str(vm)
            .unwrap()
            .to_string();
        Ok(res)
    })
}
