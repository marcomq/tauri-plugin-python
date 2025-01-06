# Tauri Plugin Python

This [tauri](https://v2.tauri.app/) plugin is supposed to make it easy to use Python as backend code.
It uses [PyO3](https://pyo3.rs) to call python from rust.
The plugin reads by default the file `src-tauri/src-python/main.py` during 
startup and runs it immediately. Python functions are then registered during initialization 
and can get called during application workflow.


| Platform | Supported |
| -------- | --------- |
| Linux    | ✓         |
| Windows  | ✓         |
| macOS    | ✓         |
| Android  | not yet   |
| iOS      | not yet   |


You might use this plugin to create simple prototype applications
and later re-write functions in rust to improve
performance, add a specific rust library or just call some 
low-level code.

Android and iOS are possible in theory but I still need to figure out how to 
cross compile python and PyO3 for iOS and android.

Also, this plugin hasn't been optimized yet for production binaries. 
The target platform therefore either needs to have libpython installed 
or you manually need to ship the shared libs together with the installer package.

## Example app

There is a sample Desktop application for Windows/Linux/MacOS using this plugin and vanilla 
Javascript in [examples/plain-javascript](https://github.com/marcomq/tauri-plugin-python/tree/main/examples/plain-javascript)

## Manual plugin installation / usage

These steps assume that you already have a basic tauri application available. Alternatively, you can immediately start with the example application.

- `$ cargo add tauri-plugin-python`
- `$ npm install tauri-plugin-python-api`
- modify `permissions:[]` in src-tauri/capabilities/default.json and add "python:default"  
- add file `src-tauri/src-python/main.py` and add python code, for example:
```python
# src-tauri/src-python/main.py
def greet_python(rust_var)
    print(rust_var)
    return str(rust_var) + " from python"
```
- add `.plugin(tauri_plugin_python::init(vec!["greet_python"))` to `tauri::Builder::default()`, usually in `src-tauri/src/lib.rs`. This will initialize the plugin and make the python function "greet_python" available from javascript.
- add javascript for python plugin in the index.html file directly or in your somewhere in your javascript application. For vanilla javascript / iife, the modules can be found in `window.__TAURI__.python`. For modern javascript:
```javascript
import { callFunction } from 'tauri-plugin-python-api'
console.log(await callFunction("greet_python", ["input value"]))
```
-> this will call the python function "greet_python" with parameter "input value". Of course, you can just pass in any available javascript value. This should work with "boolean", "integer", "double", "string", "string[]", "double[]" parameter types.

Alternatively, to have more readable code: 
```javascript
import { call, registerJs } from 'tauri-plugin-python-api'
registerJs("greet_python");
console.log(await call.greet_python("input value"));
```

## Deployment

You either need to have python installed on the target machine or ship the shared python library with your package. You also may link the python library statically - PyO3 may do this by default if it finds a static python library. In addition, you need to copy the python files so that python files are next to the binary. The file `src-python/main.py` is required for the plugin to work correctly. You may also add additional python files or use a venv environment. The included resources can be configurable in the `tauri.conf.json` file. Check the tauri and PyO3 documentation for additional info. 

## Security considerations
Generally, this plugin has been created by "security by default" concept. Python functions can only be called if registered from rust.

Keep in mind that this plugin can make it possible to run arbitrary python code. 
It is therefore highly recommended to **not make the user interface accessible by a network URL**. 

The "runPython" command is disabled by default via permissions. If enabled, it is possible to 
inject python code via javascript.
Also, the function "register" is disabled by default. If enabled, it can 
add control from javascript which functions can be called. This avoids to modify rust code when changing or adding python code.
Both functions can be enabled during development for rapid prototyping.

