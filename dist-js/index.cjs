'use strict';

var core = require('@tauri-apps/api/core');

/** Tauri Python Plugin
 * © Copyright 2024, by Marco Mengelkoch
 * Licensed under MIT License, see License file for more details
 * git clonehttps://github.com/marcomq/tauri-plugin-python
**/
let call = {}; // array of functions
async function runPython(code) {
    return await core.invoke('plugin:python|run_python', {
        payload: {
            value: code,
        },
    }).then((r) => {
        return r.value;
    });
}
/**
 * Regeisters function on server and makes it available via `call.{jsFunctionName}`
 *  @param {string} pythonFunctionCall - The python function call, can contain one dot
 *  @param {number} [numberOfArgs] - Number of arguments, used for validation in pythons, use -1 to ignore this value
 *  @param {string} [jsFunctionName] - Name that is used in javscript: "call.jsFunctionName". Must not contain dots.
 */
async function registerFunction(pythonFunctionCall, numberOfArgs, jsFunctionName) {
    if (numberOfArgs !== undefined && numberOfArgs < 0) {
        numberOfArgs = undefined;
    }
    return await core.invoke('plugin:python|register_function', {
        payload: {
            pythonFunctionCall,
            numberOfArgs
        },
    }).then((r) => {
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
async function registerJs(pythonFunctionCall, jsFunctionName) {
    if (jsFunctionName === undefined) {
        jsFunctionName = pythonFunctionCall.replaceAll(".", "_");
    }
    call[jsFunctionName] = function (...args) { return callFunction(pythonFunctionCall, args); };
}
/**
 * calling previously registered function
 */
async function callFunction(functionName, args) {
    return core.invoke('plugin:python|call_function', {
        payload: {
            functionName,
            args,
        },
    }).then((r) => {
        return r.value;
    });
}
/**
 * read variable name directly from python
 */
async function readVariable(value) {
    return core.invoke('plugin:python|read_variable', {
        payload: {
            value,
        },
    }).then((r) => {
        return r.value;
    });
}

exports.call = call;
exports.callFunction = callFunction;
exports.readVariable = readVariable;
exports.registerFunction = registerFunction;
exports.registerJs = registerJs;
exports.runPython = runPython;
