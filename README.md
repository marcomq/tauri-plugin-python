# Tauri Plugin Python

This [tauri](https://v2.tauri.app/) v2 plugin is supposed to make it easy to use Python as backend code.  
It can use either [PyO3](https://pyo3.rs) (the default) or [RustPython](https://github.com/RustPython/RustPython) as the interpreter to call python from rust.

## Choosing an interpreter: PyO3 vs. RustPython

This is the most important decision when using the plugin, so please read this before you start.

| | **PyO3 / CPython** (default) | **RustPython** |
| --- | --- | --- |
| Cargo feature | `pyo3` (enabled by default) | `rustpython` |
| Interpreter | the real CPython | a Python interpreter written in Rust |
| Library compatibility | full – anything that runs on CPython, incl. C extensions (numpy, pandas, …) | **limited** – pure-Python only, and a meaningful part of the **standard library is missing or incomplete** |
| Needs Python on the target machine | yes (libpython must be available, see below) | **no** – linked statically into your binary |
| Speed | fast (CPython) | slower |
| Deployment | more involved (ship/embed libpython) | trivial (single binary) |

> [!IMPORTANT]
> **RustPython is convenient for deployment but it is not a complete Python.** It is *not* a drop-in CPython.
> Several standard-library modules are missing, incomplete or behave differently, and some text codecs
> (e.g. `latin-1`) and modules such as `inspect` may not be importable. Code – including the plugin's own
> optional `registerFunction` argument-count check – that relies on these will silently degrade or fail on
> RustPython but work fine on PyO3. **If a library or stdlib feature doesn't work under RustPython, switch
> to the PyO3 backend before assuming it's a bug.**

The plugin uses **PyO3 by default** because of its full compatibility. The trade-off is that PyO3 needs
`libpython` to be available for the target platform, which can be more complicated to deploy (see
[Deployment](#deployment)), especially on mobile. Pick RustPython when you want a single self-contained
binary and your Python code stays within what RustPython supports.

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

You can use this plugin for fast prototypes or for (early) production code.
It might be possible that you want to use some python library or code that  
is not available for rust yet.  
In case that you want to ship production software packages, you need  
to make sure to also ship all your python code. If you use PyO3, you also need to ship libpython too.

### Switch from PyO3 to RustPython
PyO3 is the default. To get a self-contained binary that does **not** require Python on the target
machine, switch to the RustPython backend by disabling the default features and enabling `rustpython`
(keep `venv` if you rely on automatic `.venv` loading). Remember the [limitations above](#choosing-an-interpreter-pyo3-vs-rustpython).
```toml
# src-tauri/Cargo.toml
tauri-plugin-python = { version = "0.3", default-features = false, features = ["venv", "rustpython"] }
```

### PyO3 / libpython deployment
Using [PyO3](https://github.com/PyO3/pyo3) supports many more python libraries than RustPython as it is using CPython.
The trade-off is that PyO3 uses a shared libpython by default, which makes
local development easy but makes
deployment of releases more complicated.
Therefore, it may be recommended to either use [pyoxidizer](https://github.com/indygreg/PyOxidizer) to embed libpython statically 
or try to ship the dynamic libpython together with your application, for example as part
of the .venv. Check out the [PyO3 documentation](https://pyo3.rs/v0.24.2/building-and-distribution.html) for additional support.

Example of how to embed libpython statically using PyOxidizer:

This has just been tested locally on MacOS. It may be possible that this is more complicated and requires additional steps on your environment.

Install pyoxidizer `pip install pyoxidizer` in a venv and run it on bash:
```bash
pyoxidizer generate-python-embedding-artifacts src-tauri/target/pyembed
```
Then, add it to your cargo config:
```toml
# src-tauri/.cargo/config.toml
PYO3_CONFIG_FILE = { value = "target/pyembed/pyo3-build-config-file.txt", relative = true }
```
You can check if the release binary has some shared libpython references by running `otool -L tauri_app` on MacOs or `ldd tauri_app` on linux.

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
- add `"bundle": {"resources": [  "src-python/"],` to `tauri.conf.json` so that python files are bundled with your application
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
## Using a venv

Using a python venv is highly recommended when using pip dependencies (PyO3 backend).

**Put the venv at `src-python/.venv`.** The plugin auto-loads `<src-python>/.venv/lib` at startup, and
keeping the venv *inside* `src-python` is the configuration that reliably works both in `tauri dev` and in
production builds (a venv that lives outside `src-python` works in dev but commonly breaks in the bundled
app – this was the root cause for several "works in dev, not in prod" reports):

```sh
# from your src-tauri folder
python3 -m venv src-python/.venv
source src-python/.venv/bin/activate   # Windows: src-python\.venv\Scripts\activate.bat
pip install <your_lib>
```

Then ship the venv `lib` (and `include`) as resources so they sit next to `src-python` in the bundle:

`tauri.conf.json`
```json
"bundle": {
  "resources": {
    "src-python/": "src-python/"
  }
}
```
Because `.venv` lives inside `src-python/`, the single `"src-python/"` entry bundles your Python code *and*
the venv together. (If you keep the venv elsewhere, map it explicitly into `src-python/.venv/lib/` instead.)

> [!TIP]
> Pure-Python wheels are portable, but packages with compiled C extensions (numpy, pandas, pydantic-core, …)
> are platform-specific. Build the venv on / for the **same OS and architecture** you ship to, ideally in CI
> per target.

## Debugging

When a Python call fails, the plugin returns the error to the frontend (it is the rejected value of the
`callFunction` / `call.*` / `runPython` / `readVariable` promise), so the quickest first step is:

```javascript
try {
  await call.greet_python("input value");
} catch (err) {
  console.error(err); // contains the Python error message / traceback
}
```

In addition, in **development builds** (`tauri dev`, i.e. any non-release build) the plugin prints the full
error – including the Python traceback – to **stderr**, prefixed with `[tauri-plugin-python]`. Watch the
terminal running `tauri dev` to see it. Release builds do not log, so nothing leaks to end users.

The error message is also prefixed with what the plugin was doing, e.g.
`Error calling Python function 'greet_python': ...` or
`Cannot register 'greet_python': not found in Python (is it defined/imported in main.py?): ...`.

Common causes:

- **`... has not been registered yet`** – the function was never registered. Functions must be registered
  from Rust during plugin init (`init_and_register(vec!["greet_python"])`) or via the `_tauri_plugin_functions`
  list in `main.py`. The dynamic `register` command is disabled by default (see [Security](#security-considerations)).
- **`Cannot register '<name>': not found in Python`** – `main.py` didn't define/import that name, or `main.py`
  itself failed to load. Make sure `src-python/main.py` exists, is bundled as a resource (see
  [Deployment](#deployment)), and runs without errors on its own.
- **A `ModuleNotFoundError`, missing-stdlib, codec (`unknown encoding`) or `inspect` error only on RustPython** –
  this is almost always a [RustPython limitation](#choosing-an-interpreter-pyo3-vs-rustpython), not a bug.
  Try the same code under the PyO3 backend to confirm, and prefer PyO3 if you need that library.
- **Missing pip dependency** – ensure you are [using a venv](#using-a-venv) and that its `lib` folder is shipped
  as a resource next to `src-python`.

To sanity-check your Python independently of Tauri, run `python3 src-tauri/src-python/main.py` directly
(this validates it against CPython; RustPython may still differ – see the limitations above).

## Deployment

The file `src-python/main.py` is always required for the plugin to work correctly. All Python files must be
included in the tauri resource files (`tauri.conf.json`), and your bundled resource layout must mirror your
local layout so imports resolve the same way.

### RustPython
No extra steps – the interpreter is linked statically into the binary, so the target machine needs **no Python
installed**. Just bundle your `src-python/` resources. (Remember the
[stdlib limitations](#choosing-an-interpreter-pyo3-vs-rustpython).)

### PyO3 / CPython
PyO3 needs a `libpython` at runtime, which is what makes PyO3 deployment harder. Options, easiest first:

1. **Ship an embeddable / standalone Python** with the app (recommended for end-user distribution).
   Bundle a self-contained Python (e.g. [python-build-standalone](https://github.com/astral-sh/python-build-standalone)
   or the Windows embeddable zip) as a resource and point the app at it. A community project demonstrating exactly
   this with Tauri is [@Qingbao's tauri-agent](https://github.com/Qingbao/tauri-agent) – it downloads an embedded
   Python into the bundle and sets the Python path before launch, so the target needs no system Python and there is
   no fragile dynamic linking.
2. **Ship the shared libpython** next to the executable / inside the `.venv` you bundle. Check what is linked with
   `otool -L tauri_app` (macOS) or `ldd tauri_app` (Linux); a line such as
   `.../Python.framework/Versions/3.13/Python` tells you which version the target must provide at the same location.
3. **PyOxidizer / static linking** is possible but currently fragile (unmaintained `pyembed`, pyo3-version
   conflicts). Not recommended unless you specifically need it.

In all cases, also bundle your [venv](#using-a-venv) (`src-python/.venv`) for pip dependencies.

### Windows: hidden console & stdio
Tauri release builds hide the console (`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` in
`main.rs`). Without a console, Python's `stdout`/`stderr` handles are invalid, and historically a bare `print()`
could hang or crash the app on Windows release builds (issues #4/#15/#17). This plugin now **wraps Python stdio at
startup** so writes can't crash the process, and applies a **per-call timeout** (see below) so a single blocking
call can't wedge every later call. You generally no longer need to remove the `windows_subsystem` line. If you want
to *see* Python output in a release build for debugging, redirect it to a file from `main.py`, e.g.:
```python
import sys
sys.stdout = sys.stderr = open("python-debug.log", "a", buffering=1)
```

### Call timeout
Each Python call is bounded by a timeout (default **300s**) so a stuck call can't hang the app forever. Override
it with the `TAURI_PLUGIN_PYTHON_TIMEOUT_SECS` environment variable – set a larger value for long-running work, or
`0` to disable the timeout entirely. On timeout the call rejects with a timeout error; note the worker thread is
single and serial, so a call that never returns still occupies it until it finishes.

### "Works in `tauri dev` but not in the production build" checklist
This is the most common deployment problem. Check, in order:
1. Is `src-python/` (incl. `main.py`) listed under `bundle.resources` in `tauri.conf.json`? Inspect the installed
   app's resource folder to confirm the files are actually there with the same structure as locally.
2. **PyO3 + pip deps not working in prod** → your `.venv` isn't bundled or isn't at `src-python/.venv`. Move it
   inside `src-python` and bundle it (see [Using a venv](#using-a-venv)).
3. **PyO3 app won't launch at all** → missing `libpython` on the target; ship an embeddable Python or the shared
   library (above).
4. **C-extension packages (numpy, …) fail in prod** → the venv was built for a different OS/arch than you shipped.
   Rebuild it for the target platform (ideally in CI).
5. Still stuck? Temporarily build with the console visible (remove the `windows_subsystem` line on Windows, or run
   the binary from a terminal) to see startup errors, and see [Debugging](#debugging).

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
- just want to simply support rare tauri plugins  
- want to embed python code directly in your javascript
