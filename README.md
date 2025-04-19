# Tauri Plugin Python

This [tauri](https://v2.tauri.app/) v2 plugin is supposed to make it easy to use Python as backend code.  
It uses [RustPython](https://github.com/RustPython/RustPython) or alternatively [PyO3](https://pyo3.rs) as interpreter to call python from rust.

RustPython doesn't require python to be installed on the target platform and makes it  
therefore easy to deploy your production binary. Unfortunately, it has some  
compatibility issues and is slower than PyO3/CPython. PyO3 is also supported as optional Cargo feature for desktop applications.  
PyO3 uses CPython as interpreter and therefore has a wide compatibility for available python libraries.  
It isn't used as default as it requires to make libpython available for the target platform,  
which can be complicated, especially for mobile targets.

The plugin reads by default the file `src-tauri/src-python/main.py` during  
startup and runs it immediately. Make sure to add all your python source as tauri resource,  
so it is shipped together with your production binaries. Python functions are all registered during plugin initialization  
and can get called during application workflow.

| Platform | Supported |
| -------- | --------- |
| Linux    | ✓         |
| Windows  | ✓         |
| MacOS    | ✓         |
| Android  | x*        |
| iOS      | ✓*        |

`x*` There is currently a known issue on tauri+android that prevents reading files.  
https://github.com/tauri-apps/tauri/issues/11823  
So python code cannot be read on android right now. Android is going to be supported as soon as reading resource files will be fixed.

`✓*` Linux, Windows and MacOS support PyO3 and RustPython as interpreter. Android and iOS  
currently only support RustPython.  
Android and iOS might also be able to run with PyO3 in theory but would require to have CPython  
to be compiled for the target platform. I still need to figure out how to  
cross compile python and PyO3 for iOS and Android. Ping me if you know how to do that.

You can use this plugin for fast prototypes or for production code.  
It might be possible that you want to use some python library or code that  
is not available for rust yet.  
In case that you want to ship production software packages, you need  
to make sure to also ship all your python code. If you use PyO3, you also need to ship libpython too.

### Switch from RustPython to PyO3

```toml
# src-tauri/Cargo.toml
tauri-plugin-python = { version="0.3", features = ["pyo3"] }
```

## Example app

There is a sample Desktop application for Windows/Linux/MacOS using this plugin and vanilla  
Javascript in [examples/plain-javascript](https://github.com/marcomq/tauri-plugin-python/tree/main/examples/plain-javascript).

## Add the plugin to an existing tauri application

These steps assume that you already have a basic tauri application available. Alternatively, you can immediately start with the application in "example" directory.

- run `npm run tauri add python`
- add `src-tauri/src-python/main.py` and modify it according to your needs, for example add  
```python
# src-tauri/src-python/main.py
_tauri_plugin_functions = ["greet_python"]  # make "greet_python" callable from UI
def greet_python(rust_var):
    return str(rust_var) + " from python"
```
- add `"bundle": {"resources": [  "src-python/**/*"],` to `tauri.conf.json` so that python files are bundled with your application
- add the plugin in your js, so  
   - add `import { callFunction } from 'tauri-plugin-python-api'`  
   - add `outputEl.textContent = await callFunction("greet_python", [value])` to get the output of the python function `greet_python` with parameter of js variable `value`

Check the examples for alternative function calls and code sugar.

Tauri events and calling js from python is currently not supported yet. You would need to use rust for that.

## Alternative manual plugin installation

- `$ cargo add tauri-plugin-python`
- `$ npm install tauri-plugin-python-api`
- modify `permissions:[]` in src-tauri/capabilities/default.json and add "python:default"  
- add file `src-tauri/src-python/main.py` and add python code, for example:
```python
# src-tauri/src-python/main.py
def greet_python(rust_var):
    return str(rust_var) + " from python"
```
- add `.plugin(tauri_plugin_python::init_and_register(vec!["greet_python"]))` to `tauri::Builder::default()`, usually in `src-tauri/src/lib.rs`. This will initialize the plugin and make the python function "greet_python" available from javascript.
- add javascript for python plugin in the index.html file directly or somewhere in your javascript application. For vanilla javascript / iife, the modules can be found in `window.__TAURI__.python`. For modern javascript:
```javascript
import { callFunction } from 'tauri-plugin-python-api'
console.log(await callFunction("greet_python", ["input value"]))
```
→ this will call the python function "greet_python" with parameter "input value". Of course, you can just pass in any available javascript value. This should work with "boolean", "integer", "double", "string", "string[]", "double[]" parameter types.

Alternatively, to have more readable code:  
```javascript
import { call, registerJs } from 'tauri-plugin-python-api'
registerJs("greet_python");
console.log(await call.greet_python("input value"));
```

## Deployment

You either need to have python installed on the target machine or ship the shared  
python library with your package. You also may link the python library statically - PyO3  
may do this by default if it finds a static python library. In addition, you need  
to copy the python files so that python files are next to the binary.  

The file `src-python/main.py` is required for the plugin to work correctly.  
You may also add additional python files or use a venv environment.  
The included resources can be configurable in the `tauri.conf.json` file.  

Check the tauri and PyO3 documentation for additional info.

## Security considerations

By default, this plugin cannot call arbitrary python code. Python functions can only be called if registered from rust during plugin initialization.  
It may still be possible to read values from python. This can be prevented via additional tauri permissions.

Keep in mind that this plugin could make it possible to run arbitrary python code when using all allow permissions.  
It is therefore highly recommended to **make sure the user interface is not accessible by a network URL** in production.

The "runPython" command is disabled by default via permissions. If enabled, it is possible to  
inject python code directly via javascript.  
Also, the function "register" is disabled by default. If enabled, it can  
add control from javascript which functions can be called. This avoids to modify rust code when changing or adding python code.  
Both functions can be enabled during development for rapid prototyping.

## Alternatives

If you already know that you just want to develop completely in python, you might want to take a look at [pytauri](https://github.com/WSH032/pytauri).  
It is a different approach to have all tauri functionality completely in python.

This approach here with tauri-plugin-python is more lightweight and it is for you, if you  
- still want to write rust code  
- already have a tauri application and just need a specific python library  
- just want to simply support rare custom plugins  
- want to embed python code directly in your javascript
