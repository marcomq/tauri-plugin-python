# Tauri Plugin Python

This [tauri](https://v2.tauri.app/) v2 plugin is supposed to make it easy to use Python as backend code.  
It can use either [PyO3](https://pyo3.rs) (the default) or [RustPython](https://github.com/RustPython/RustPython) as the interpreter to call python from rust.  
The companion [`tauri-plugin-python-cli`](#deployment-tool-tauri-plugin-python-cli) bundles a standalone Python into your app, so releases run on machines without Python.

> **Requirements:** Tauri **v2**, a recent stable Rust toolchain and, for the default PyO3 backend, a CPython
> interpreter on the development machine. Released apps ship their own Python, see [Deployment](#deployment).
> The [example apps](#example-app) are the canonical, always-current reference if anything below drifts out of date.

## Quick start

From an existing Tauri v2 app (or start from the [example apps](#example-app)). This uses the default **PyO3**
backend and works immediately in `tauri dev`:

```sh
npm run tauri add python
```

Create `src-tauri/src-python/main.py`:
```python
# src-tauri/src-python/main.py
_tauri_plugin_functions = ["greet_python"]  # make this callable from the UI

def greet_python(name):
    return f"Hello {name} from Python"
```

Make sure Python is bundled as a resource in `src-tauri/tauri.conf.json`:
```json
"bundle": { "resources": ["src-python/"] }
```

Call it from your frontend:
```javascript
import { callFunction } from 'tauri-plugin-python-api'
console.log(await callFunction("greet_python", ["world"])) // "Hello world from Python"
```

That's it for development. Before you ship a release, bundle a standalone Python with the
[CLI](#pyo3--cpython) and read [Deployment](#deployment):
```sh
npx tauri-plugin-python-cli setup
```
If you want a single self-contained binary without any Python files, see [RustPython](#switch-from-pyo3-to-rustpython).

## Deployment tool: tauri-plugin-python-cli

[`tauri-plugin-python-cli`](https://github.com/marcomq/tauri-plugin-python/tree/main/cli) is an npm package that takes care of the hard part of shipping a
PyO3 app: a Python that runs on the user's machine. It needs Node.js 18+ and nothing else; run it in your project
root or in `src-tauri`:

| Command | What it does |
| --- | --- |
| `npx tauri-plugin-python-cli setup` | downloads a relocatable Python into `src-tauri/pyembed/`, installs `src-python/requirements.txt` into it and configures cargo and tauri to bundle and link it |
| `npx tauri-plugin-python-cli doctor` | checks entry point, resources, bundled Python and, after a build, which `libpython` the binary loads; `--fix` repairs the RustPython lockfile issue |
| `npx tauri-plugin-python-cli sign` | macOS: signs the bundled Python for notarization |

Details in [Deployment → PyO3 / CPython](#pyo3--cpython) and in the step-by-step guides for
[macOS](https://github.com/marcomq/tauri-plugin-python/blob/main/docs/deploy-macos.md), [Linux](https://github.com/marcomq/tauri-plugin-python/blob/main/docs/deploy-linux.md) and [Windows](https://github.com/marcomq/tauri-plugin-python/blob/main/docs/deploy-windows.md).

## Choosing an interpreter: PyO3 vs. RustPython

This is the most important decision when using the plugin, so please read this before you start.

| | **PyO3 / CPython** (default) | **RustPython** |
| --- | --- | --- |
| Cargo feature | `pyo3` (enabled by default) | `rustpython` |
| Interpreter | the real CPython | a Python interpreter written in Rust |
| Library compatibility | full – anything that runs on CPython, incl. C extensions (numpy, pandas, …) | **limited** – pure-Python code and the pure-Python stdlib; no C extensions, no `sqlite3`, no `ssl` |
| `async def` functions | ✓ | ✗ |
| Compiled `main` (Nuitka, `.pyc`) | ✓ | ✗ |
| Needs Python on the target machine | no, if you bundle one with the [CLI](#pyo3--cpython) | **no** – interpreter and stdlib are compiled into your binary |
| Speed | fast (CPython) | slower |
| Deployment | `tauri-plugin-python-cli setup` before `tauri build` | trivial (single binary) |

> [!IMPORTANT]
> **RustPython is convenient for deployment but it is not a complete Python.** The pure-Python part of the
> standard library (`json`, `typing`, `inspect`, `dataclasses`, `pathlib`, `logging`, …) is compiled into the
> binary, but everything that needs CPython C extensions is missing: `sqlite3`, `ssl`, and third-party packages
> such as numpy or pydantic-core. `async def` functions can't be called yet. **If a library doesn't work under
> RustPython, switch to the PyO3 backend before assuming it's a bug.**

The plugin uses **PyO3 by default** because of its full compatibility. The trade-off is that the app needs a
`libpython` at runtime; [`tauri-plugin-python-cli`](#pyo3--cpython) bundles one for desktop targets. Mobile
currently needs RustPython. Pick RustPython when you want a single self-contained binary and your Python code
stays within what RustPython supports.

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
to make sure to also ship all your python code. If you use PyO3, you also need to ship a Python, see [Deployment](#deployment).

### Switch from PyO3 to RustPython
PyO3 is the default. To get a self-contained binary that does **not** require Python on the target
machine, switch to the RustPython backend by disabling the default features and enabling `rustpython`
(keep `venv` if you rely on automatic `.venv` loading). Remember the [limitations above](#choosing-an-interpreter-pyo3-vs-rustpython).
```toml
# src-tauri/Cargo.toml
tauri-plugin-python = { version = "0.4", default-features = false, features = ["venv", "rustpython"] }
```

`rustpython-stdlib` 0.5 pulls in `malachite-bigint` twice (0.9 directly and 0.14
via `pymath`) on a fresh lockfile, which fails to compile with type mismatches.
Pin the second copy to 0.9.2 once after adding the dependency:

```sh
cargo update -p malachite-bigint@0.14.0 --precise 0.9.2
```

`npx tauri-plugin-python-cli doctor --fix` detects and applies this pin. See also the
[async_py README](https://github.com/marcomq/async_py#rustpython).

## Async functions

`async def` functions can be registered and called like normal ones; the plugin awaits them and returns the
result (PyO3 backend only):
```python
import asyncio
_tauri_plugin_functions = ["fetch_answer"]

async def fetch_answer(question):
    await asyncio.sleep(0.1)
    return f"42 is the answer to {question}"
```

The check happens at registration with `inspect.iscoroutinefunction`. A plain function that only *returns* a
coroutine (e.g. wrapped by a sync decorator) isn't awaited; write such wrappers as `async def`.

## Compiled entry point (Nuitka, `.pyc`)

If `src-python/main.py` doesn't exist, the plugin imports a compiled module named `main` from `src-python/`
instead: `main.cpython-313-darwin.so`, `main.pyd`, `main.pyc`, …. This lets you ship your code compiled, e.g.
with [Nuitka](https://nuitka.net/):
```sh
python -m nuitka --module src-tauri/src-python/main.py --output-dir=src-tauri/src-python
```
Compiled modules only load into the exact Python version they were built for, so build them with the same
Python version you bundle (see `src-python/.python-version` below). PyO3 backend only.

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

> [!NOTE]
> For releases, prefer `src-python/requirements.txt`: `tauri-plugin-python-cli setup` installs it into the
> bundled Python, which works on any machine. A `src-python/.venv` is bundled along with `src-python/` and
> loaded in addition, but only if it was created with the same Python version; otherwise the plugin prints a
> warning and ignores it.

Using a python venv is highly recommended when using pip dependencies during development (PyO3 backend).

**Put the venv at `src-python/.venv`.** At startup the plugin adds its `site-packages` to `sys.path` if it
matches the running Python version (otherwise it prints a warning), and
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
- **A `ModuleNotFoundError` (e.g. `sqlite3`, `ssl` or a C extension package) only on RustPython** –
  this is almost always a [RustPython limitation](#choosing-an-interpreter-pyo3-vs-rustpython), not a bug.
  Try the same code under the PyO3 backend to confirm, and prefer PyO3 if you need that library.
- **Missing pip dependency** – add it to `src-python/requirements.txt` and run `tauri-plugin-python-cli setup`
  again, or, in development, install it into your [venv](#using-a-venv).
- **Something else is off** – run `npx tauri-plugin-python-cli doctor`; it checks entry point, resources and the
  bundled Python, and after a build also which `libpython` the binary loads.

To sanity-check your Python independently of Tauri, run `python3 src-tauri/src-python/main.py` directly
(this validates it against CPython; RustPython may still differ – see the limitations above).

## Deployment

The file `src-python/main.py` is always required for the plugin to work correctly. All Python files must be
included in the tauri resource files (`tauri.conf.json`), and your bundled resource layout must mirror your
local layout so imports resolve the same way.

The `bundle.resources` entry can be written as a list or as a source→target map; both are equivalent:
```json
"bundle": { "resources": ["src-python/"] }
// equivalent to:
"bundle": { "resources": { "src-python/": "src-python/" } }
```

After building, confirm the files actually landed in the installed app's resource folder (this is where the plugin
looks at runtime):

| Platform | Resource folder (relative to the app) |
| -------- | ------------------------------------- |
| macOS    | `<App>.app/Contents/Resources/`       |
| Linux    | next to the executable, e.g. `usr/lib/<app>/` (AppImage/deb) |
| Windows  | next to the `.exe` (the install dir)  |

You should find `src-python/main.py` (and `.venv/` if you bundled one) there with the same structure as locally.

### RustPython
No extra steps – the interpreter is linked statically into the binary, so the target machine needs **no Python
installed**. Just bundle your `src-python/` resources. (Remember the
[stdlib limitations](#choosing-an-interpreter-pyo3-vs-rustpython).)

### PyO3 / CPython
PyO3 links `libpython` at build time, and the OS loader looks for exactly that library at runtime. Without
further steps the app therefore only starts on machines that have the same Python at the same location as the
build machine. [`tauri-plugin-python-cli`](https://github.com/marcomq/tauri-plugin-python/tree/main/cli) bundles a relocatable
[python-build-standalone](https://github.com/astral-sh/python-build-standalone) interpreter instead:

```sh
npx tauri-plugin-python-cli setup     # once, after cloning, and when requirements change
npm run tauri build
npx tauri-plugin-python-cli doctor    # optional: checks which libpython the built app loads
```

Step-by-step guides: **[macOS](https://github.com/marcomq/tauri-plugin-python/blob/main/docs/deploy-macos.md)** · **[Linux](https://github.com/marcomq/tauri-plugin-python/blob/main/docs/deploy-linux.md)** ·
**[Windows](https://github.com/marcomq/tauri-plugin-python/blob/main/docs/deploy-windows.md)**

`setup` downloads Python into `src-tauri/pyembed/`,
installs `src-python/requirements.txt` into it, and configures the build:

- `src-tauri/.cargo/config.toml` gets a managed block with `PYO3_CONFIG_FILE` (link against the bundled
  `libpython`) and, on macOS and Linux, an rpath so the binary finds it inside the app bundle.
- `tauri.<os>.conf.json` registers the Python runtime as resources. Your `bundle.resources` in
  `tauri.conf.json` is converted to the object form so tauri can merge both.
- `/pyembed/` is added to `src-tauri/.gitignore`; it is machine specific.

Commit the config changes and run `setup` after every clone and in CI before `tauri build`. Without it, the build fails with `failed to open PyO3 config file at …/pyembed/pyo3-config.txt`,
because the committed `.cargo/config.toml` points to a file in the uncommitted `pyembed/`.

The Python version comes from `src-python/.python-version` (e.g. `3.13`, same format as `uv python pin`). If the
file is missing, `setup` takes the newest Python that reached its x.y.2 bugfix release (`stable`) and writes its
minor version there. Commit that file so the team and CI bundle the same version. To change it, edit the file or
run `setup --python 3.14` (also an exact `3.14.2`, `stable` or `latest`), which updates the file.

At startup the plugin finds `pyembed/python` in the resources (or in `src-tauri` during `tauri dev`) and uses
it as `PYTHONHOME`. Without `pyembed/` the plugin uses the Python it was linked against, as before.

Notes per platform:

- **Build on each target OS.** The bundled Python is native code; build the macOS app on macOS, the Linux
  packages on Linux and the Windows installer on Windows, e.g. in a CI matrix. Run `setup` in CI before
  `tauri build`; downloads are cached in `~/.cache/tauri-plugin-python-cli`.
- **macOS:** notarization requires every native file in the app to be signed. Tauri only signs the main binary,
  so run `npx tauri-plugin-python-cli sign` (uses `APPLE_SIGNING_IDENTITY`) **before** `tauri build`; it signs
  the files in `src-tauri/pyembed`, which tauri then copies into the app.
- **Windows:** `python3XX.dll` is placed next to the `.exe`, the rest under `pyembed/`.
- **Linux:** the runtime lands in `/usr/lib/<productName>/pyembed/`.
- `RUSTFLAGS` in the environment replaces the rpaths from `.cargo/config.toml`; `doctor` warns about it.

If you prefer to wire it up yourself: everything `setup` writes is plain tauri and cargo configuration, see
[cli/README.md](https://github.com/marcomq/tauri-plugin-python/tree/main/cli) and [PyO3 build & distribution](https://pyo3.rs/main/building-and-distribution).

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
   Debug builds fall back to `src-python/` in the current directory, release builds don't.
2. **PyO3 app won't launch at all, or only on your machine** → the app has no bundled Python. Run
   `npx tauri-plugin-python-cli setup` before `tauri build` (see [PyO3 / CPython](#pyo3--cpython)), then
   `npx tauri-plugin-python-cli doctor`.
3. **PyO3 + pip deps not working in prod** → list them in `src-python/requirements.txt` and run `setup` again.
4. **C-extension packages (numpy, …) fail in prod** → they were installed for a different OS/arch than you
   shipped. Run `setup` and `tauri build` on each target platform (ideally in CI).
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
