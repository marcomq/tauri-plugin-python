/** Tauri Python Plugin
 * © Copyright 2024, by Marco Mengelkoch
 * Licensed under MIT License, see License file for more details
 * git clonehttps://github.com/marcomq/tauri-python-plugin
**/

import { invoke } from '@tauri-apps/api/core'

export let call: { [index: string]: Function } = {}; // array of functions

export async function runPython(code: string): Promise<string> {
  return await invoke<{ value: string }>('plugin:python|run_python', {
    payload: {
      value: code,
    },
  }).then((r: any) => {
    return r.value;
  });
}

export async function registerFunction(functionName: string, numberOfArgs?: number): Promise<string> {
  return await invoke<{ value: string }>('plugin:python|register_function', {
    payload: {
      functionName,
      numberOfArgs,
    },
  }).then((r:any) => {
    call[functionName] = function (...args: any[]) { return callFunction(functionName, args) };
    return r.value;
  });
}

export async function callFunction(functionName: string, args: any[]): Promise<string> {
  return invoke<{ value: string }>('plugin:python|call_function', {
    payload: {
      functionName,
      args,
    },
  }).then((r:any) => {
    return r.value;
  });
}

export async function readVariable(value: string): Promise<string> {
  return invoke<{ value: string }>('plugin:python|read_variable', {
    payload: {
      value,
    },
  }).then((r:any) => {
    return r.value;
  });
}