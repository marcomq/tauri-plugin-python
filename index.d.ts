/** Tauri Python Plugin
 * © Copyright 2024, by Marco Mengelkoch
 * Licensed under MIT License, see License file for more details
 * git clonehttps://github.com/marcomq/tauri-python-plugin
**/
export declare let py: {
    [index: string]: Function;
};
export declare function runPython(code: string): Promise<string>;
export declare function registerFunction(functionName: string, numberOfArgs?: number): Promise<string>;
export declare function callFunction(functionName: string, args: any[]): Promise<string>;
export declare function readVariable(value: string): Promise<string>;
