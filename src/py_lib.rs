//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use std::sync::atomic::AtomicBool;
use std::{collections::HashSet, sync::Mutex};

use rustpython_vm::py_serde;

use lazy_static::lazy_static;

use crate::{models::*, Error};

fn create_globals() -> rustpython_vm::scope::Scope {
    rustpython_vm::Interpreter::without_stdlib(Default::default())
        .enter(|vm| vm.new_scope_with_builtins())
}

lazy_static! {
    static ref INIT_BLOCKED: AtomicBool = false.into();
    static ref FUNCTION_MAP: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    static ref GLOBALS: rustpython_vm::scope::Scope = create_globals();
}

pub fn init_python(code: String) -> crate::Result<()> {
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

pub fn run_python(payload: StringRequest) -> crate::Result<()> {
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
pub fn register_function(payload: RegisterRequest) -> crate::Result<()> {
    register_function_str(payload.python_function_call, payload.number_of_args)
}

pub fn register_function_str(fn_name: String, number_of_args: Option<u8>) -> crate::Result<()> {
    if INIT_BLOCKED.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("Cannot register after function called".into());
    }
    rustpython_vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
        GLOBALS
            .globals
            .get_item(&fn_name, vm)
            .expect(&format!("Function {fn_name} not found"));
        FUNCTION_MAP.lock().unwrap().insert(fn_name);
        Ok(())
    })
}
pub fn call_function(payload: RunRequest) -> crate::Result<String> {
    INIT_BLOCKED.store(true, std::sync::atomic::Ordering::Relaxed);
    let function_name = payload.function_name;
    if FUNCTION_MAP.lock().unwrap().get(&function_name).is_none() {
        return Err(Error::String(format!(
            "Function {function_name} has not been registered yet"
        )));
    }
    rustpython_vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
        let posargs: Vec<_> = payload
            .args
            .into_iter()
            .map(|value| py_serde::deserialize(vm, value).unwrap())
            .collect();
        let res = GLOBALS
            .globals
            .get_item(&function_name, vm)?
            .call(posargs, vm)?
            .str(vm)?
            .to_string();
        Ok(res)
    })
}

pub fn read_variable(payload: StringRequest) -> crate::Result<String> {
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
