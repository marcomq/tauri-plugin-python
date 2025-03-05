if ('__TAURI__' in window) {
var __TAURI_PLUGIN_PYTHON_API__ = (function (exports) {
    'use strict';

    /******************************************************************************
    Copyright (c) Microsoft Corporation.

    Permission to use, copy, modify, and/or distribute this software for any
    purpose with or without fee is hereby granted.

    THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
    REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
    AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
    INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
    LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
    OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
    PERFORMANCE OF THIS SOFTWARE.
    ***************************************************************************** */
    /* global Reflect, Promise, SuppressedError, Symbol, Iterator */


    typeof SuppressedError === "function" ? SuppressedError : function (error, suppressed, message) {
        var e = new Error(message);
        return e.name = "SuppressedError", e.error = error, e.suppressed = suppressed, e;
    };

    /**
     * Sends a message to the backend.
     * @example
     * ```typescript
     * import { invoke } from '@tauri-apps/api/core';
     * await invoke('login', { user: 'tauri', password: 'poiwe3h4r5ip3yrhtew9ty' });
     * ```
     *
     * @param cmd The command name.
     * @param args The optional arguments to pass to the command.
     * @param options The request options.
     * @return A promise resolving or rejecting to the backend response.
     *
     * @since 1.0.0
     */
    async function invoke(cmd, args = {}, options) {
        return window.__TAURI_INTERNALS__.invoke(cmd, args, options);
    }

    /** Tauri Python Plugin
     * © Copyright 2024, by Marco Mengelkoch
     * Licensed under MIT License, see License file for more details
     * git clone https://github.com/marcomq/tauri-plugin-python
    **/
    let call = {}; // array of functions
    async function runPython(code) {
        return await invoke('plugin:python|run_python', {
            payload: {
                value: code,
            },
        }).then((r) => {
            return r.value;
        });
    }
    /**
     * Registers function on server and makes it available via `call.{jsFunctionName}`
     *  @param {string} pythonFunctionCall - The python function call, can contain one dot
     *  @param {number} [numberOfArgs] - Number of arguments, used for validation in python, use -1 to ignore this value
     *  @param {string} [jsFunctionName] - Name that is used in javascript: "call.jsFunctionName". Must not contain dots.
     */
    async function registerFunction(pythonFunctionCall, numberOfArgs, jsFunctionName) {
        if (numberOfArgs !== undefined && numberOfArgs < 0) {
            numberOfArgs = undefined;
        }
        return await invoke('plugin:python|register_function', {
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
     *  @param {string} [jsFunctionName] - Name that is used in javascript: "call.jsFunctionName". Must not contain dots.
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
        return invoke('plugin:python|call_function', {
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
        return invoke('plugin:python|read_variable', {
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

    return exports;

})({});
Object.defineProperty(window.__TAURI__, 'python', { value: __TAURI_PLUGIN_PYTHON_API__ }) }
