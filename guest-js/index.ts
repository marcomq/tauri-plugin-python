/** Tauri Python Plugin
 * © Copyright 2024, by Marco Mengelkoch
 * Licensed under MIT License, see License file for more details
 * git clonehttps://github.com/marcomq/tauri-plugin-python
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

/** 
 * Regeisters function on server and makes it available via `call.{jsFunctionName}`
 *  @param {string} pythonFunctionCall - The python function call, can contain one dot
 *  @param {number} [numberOfArgs] - Number of arguments, used for validation in pythons, use -1 to ignore this value
 *  @param {string} [jsFunctionName] - Name that is used in javscript: "call.jsFunctionName". Must not contain dots.
 */
export async function registerFunction(
  pythonFunctionCall: string,
  numberOfArgs?: number,
  jsFunctionName?: string): Promise<string> {
  if (numberOfArgs !== undefined && numberOfArgs < 0) {
    numberOfArgs = undefined;
  }
  return await invoke<{ value: string }>('plugin:python|register_function', {
    payload: {
      pythonFunctionCall,
      numberOfArgs
    },
  }).then((r: any) => {
    registerJs(pythonFunctionCall, jsFunctionName);
    return r.value;
  });
}



/** 
 * No server invokation - assumes that function has already been registered server-side
 * Makes function available as `call.{jsFunctionName}`
 *  @param {string} pythonFunctionCall - The python function call, can contain one dot
 *  @param {string} [jsFunctionName] - Name that is used in javscript: "call.jsFunctionName". Must not contain dots.
 */
export async function registerJs(pythonFunctionCall: string, jsFunctionName?: string) {
  if (jsFunctionName === undefined) {
    jsFunctionName = pythonFunctionCall.replaceAll(".", "_");
  }
  call[jsFunctionName] = function (...args: any[]) { return callFunction(pythonFunctionCall, args) };
}

/**
 * calling previously registered function 
 */
export async function callFunction(functionName: string, args: any[]): Promise<string> {
  return invoke<{ value: string }>('plugin:python|call_function', {
    payload: {
      functionName,
      args,
    },
  }).then((r: any) => {
    return r.value;
  });
}

/**
 * read variable name directly from python
 */
export async function readVariable(value: string): Promise<string> {
  return invoke<{ value: string }>('plugin:python|read_variable', {
    payload: {
      value,
    },
  }).then((r: any) => {
    return r.value;
  });
}