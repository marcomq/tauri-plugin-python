# Tauri Plugin Python

This [tauri](https://v2.tauri.app/) plugin is supposed to make it easy to use Python as backend code.
It uses [RustPython](https://github.com/RustPython/RustPython) as interpreter to call python from rust.
RustPython doesn't require python to be installed on the target platform and makes it 
therefore easy to deploy your production binary. Unfortunately, it has some 
compatibility issues and is slower than PyO3/CPython. [PyO3](https://pyo3.rs) is also supported as optional Cargo feature for desktop applications. 
PyO3 uses CPython as interpreter and therefore has a much better compatibility for python libraries.
It isn't used as default as it requires to make libpython available for the target platform,
which can be complicated, especially for mobile targets.

The plugin reads by default the file `src-tauri/src-python/main.py` during 
startup and runs it immediately. Make sure to add all your python source as tauri resource,
so it is shipped together with your productioon binaries. Python functions are all registered during plugin initialization 
and can get called during application workflow.


| Platform | Supported |
| -------- | --------- |
| Linux    | ✓         |
| Windows  | ✓         |
| MacOS    | ✓         |
| Android  | ✓*        |
| iOS      | ✓*        |

`*` Linux, Windows and MacOS support PyO3 and RustPython as interpreter. Android and IOS
currently only supports RustPython. 
Android and iOS might also be able to run PyO3 in theory but require to have CPython
to be compiled for the target platform. I still need to figure out how to 
cross compile python and PyO3 for iOS and Android. Ping me if you know how to do that.

You might use this plugin to create simple prototype applications
and later re-write functions in rust to improve
performance, add a specific rust library or just call some 
low-level code.

## Example app

There is a sample Desktop application for Windows/Linux/MacOS using this plugin and vanilla 
Javascript in [examples/plain-javascript](https://github.com/marcomq/tauri-plugin-python/tree/main/examples/plain-javascript).


## Add the plugin to an existing tauri application
- run `npm run tauri add python`
- add `src-tauri/src-python/main.py` and modify it acording to your needs, for example add `def greet_python(intput): return str(input) + " from python"`
- modify `src-tauri/src/lib.rs` and change `.plugin(tauri_plugin_python::init())` to `.plugin(tauri_plugin_python::init(["greet_python"]))`; make sure you list all python functions you 
want to call
- add `"bundle": {"resources": [  "src-python/**/*"],` to `tauri.conf.json` so that python files are bundled with your application
- add the plugin in your js, so 
   - add `import { callFunction } from 'tauri-plugin-python-api'` 
   - add `outputEl.textContent = await tauri.python.callFunction("greet_python", [value])` to get the output of the python function `greet_python` with parameter of js variable `value`

Check the examples for alternative function calls and code sugar.

Tauri events and calling js from python is currently not supported yet. You would need to use rust for that.

## Security considerations
Generally, this plugin has been created by "security by default" concept. Python functions can onl be called if registered from rust during plugin initialization.

Keep in mind that this plugin could make it possible to run arbitrary python code. 
It is therefore highly recommended to **not make the user interface accessible by a network URL**. 

The "runPython" command is disabled by default via permissions. If enabled, it is possible to 
inject python code via javascript.
Also, the function "register" is disabled by default. If enabled, it can 
add control from javascript which functions can be called. This avoids to modify rust code when changing or adding python code.
Both functions can be enabled during development for rapid prototyping.

