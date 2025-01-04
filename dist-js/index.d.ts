/** Tauri Python Plugin
 * © Copyright 2024, by Marco Mengelkoch
 * Licensed under MIT License, see License file for more details
 * git clonehttps://github.com/marcomq/tauri-plugin-python
**/
export declare let call: {
    [index: string]: Function;
};
export declare function runPython(code: string): Promise<string>;
/**
 * Regeisters function on server and makes it available via `call.{jsFunctionName}`
 *  @param {string} pythonFunctionCall - The python function call, can contain one dot
 *  @param {number} [numberOfArgs] - Number of arguments, used for validation in pythons, use -1 to ignore this value
 *  @param {string} [jsFunctionName] - Name that is used in javscript: "call.jsFunctionName". Must not contain dots.
 */
export declare function registerFunction(pythonFunctionCall: string, numberOfArgs?: number, jsFunctionName?: string): Promise<string>;
/**
 * No server invokation - assumes that function has already been registered server-side
 * Makes function available as `call.{jsFunctionName}`
 *  @param {string} pythonFunctionCall - The python function call, can contain one dot
 *  @param {string} [jsFunctionName] - Name that is used in javscript: "call.jsFunctionName". Must not contain dots.
 */
export declare function registerJs(pythonFunctionCall: string, jsFunctionName?: string): Promise<void>;
/**
 * calling previously registered function
 */
export declare function callFunction(functionName: string, args: any[]): Promise<string>;
/**
 * read variable name directly from python
 */
export declare function readVariable(value: string): Promise<string>;
