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

pub fn run_python(payload: StringRequest) -> crate::Result<()> {
    run_python_internal(payload.value, "<embedded>".into())
}

pub fn run_python_internal(code: String, filename: String) -> crate::Result<()> {
    rustpython_vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
        let code_obj = vm
            .compile(&code, rustpython_vm::compiler::Mode::Exec, filename)
            .map_err(|err| vm.new_syntax_error(&err, Some(&code)))?;
        vm.run_code_obj(code_obj, GLOBALS.clone())
    })?;
    Ok(())
}

pub fn register_function(payload: RegisterRequest) -> crate::Result<()> {
    register_function_str(payload.python_function_call, payload.number_of_args)
}

pub fn register_function_str(
    function_name: String,
    number_of_args: Option<u8>,
) -> crate::Result<()> {
    if INIT_BLOCKED.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("Cannot register after function called".into());
    }
    rustpython_vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
        let var_dot_split: Vec<&str> = function_name.split(".").collect();
        let func = GLOBALS
            .globals
            .get_item(var_dot_split[0], vm)
            .unwrap_or_else(|_| {
                panic!("Cannot find '{}' in globals", var_dot_split[0]);
            });
        if var_dot_split.len() > 2 {
            func.get_attr(&vm.ctx.new_str(var_dot_split[1]), vm)
                .unwrap()
                .get_attr(&vm.ctx.new_str(var_dot_split[2]), vm)
                .unwrap();
        } else if var_dot_split.len() > 1 {
            func.get_attr(&vm.ctx.new_str(var_dot_split[1]), vm)
                .unwrap_or_else(|_| {
                    panic!(
                        "Cannot find sub function '{}' in '{}'",
                        var_dot_split[1], var_dot_split[0]
                    );
                });
        }

        if let Some(num_args) = number_of_args {
            let py_analyze_sig = format!(
                r#"
from inspect import signature
if len(signature({}).parameters) != {}:
    raise Exception("Function parameters don't match in 'registerFunction'")
"#,
                function_name, num_args
            );

            let code_obj = vm
                .compile(
                    &py_analyze_sig,
                    rustpython_vm::compiler::Mode::Exec,
                    "<embedded>".to_owned(),
                )
                .map_err(|err| vm.new_syntax_error(&err, Some(&py_analyze_sig)))?;
            vm.run_code_obj(code_obj, GLOBALS.clone())
                .unwrap_or_else(|_| {
                    panic!("Number of args doesn't match signature of {function_name}.")
                });
        }
        // dbg!(format!("Added '{function_name}'"));
        FUNCTION_MAP.lock().unwrap().insert(function_name);
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
        let var_dot_split: Vec<&str> = function_name.split(".").collect();
        let func = GLOBALS.globals.get_item(var_dot_split[0], vm)?;
        Ok(if var_dot_split.len() > 2 {
            func.get_attr(&vm.ctx.new_str(var_dot_split[1]), vm)?
                .get_attr(&vm.ctx.new_str(var_dot_split[2]), vm)?
        } else if var_dot_split.len() > 1 {
            func.get_attr(&vm.ctx.new_str(var_dot_split[1]), vm)?
        } else {
            func
        }
        .call(posargs, vm)?
        .str(vm)?
        .to_string())
    })
}

pub fn read_variable(payload: StringRequest) -> crate::Result<String> {
    rustpython_vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
        let var_dot_split: Vec<&str> = payload.value.split(".").collect();
        let var = GLOBALS.globals.get_item(var_dot_split[0], vm)?;
        Ok(if var_dot_split.len() > 2 {
            var.get_attr(&vm.ctx.new_str(var_dot_split[1]), vm)?
                .get_attr(&vm.ctx.new_str(var_dot_split[2]), vm)?
        } else if var_dot_split.len() > 1 {
            var.get_attr(&vm.ctx.new_str(var_dot_split[1]), vm)?
        } else {
            var
        }
        .str(vm)?
        .to_string())
    })
}
