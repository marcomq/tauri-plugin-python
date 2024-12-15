# Tauri + Python Plugin + Vanilla

This template should help get you started developing with Tauri and Python Plugin 
and Vanilla Javascript

## Modifications on default template to add plugin:
- add `tauri-plugin-python` to Cargo.toml 
- add `tauri-plugin-python-api` to package.json
- modify `permissions:[]` in src-tauri/capabilities/default.json and add "python:default"  
- modify `src-tauri/src-python/main.py` and add python code, for example `def greet_python(..`
- add `.plugin(tauri_plugin_python::init())` to `src-tauri/src/lib.rs`
- include javascript for python plugin for example by adding `<script type="module" src="/tauri-python-plugin-api/index.iife.js" defer></script>`
- register python functions in javascript by calling `registerFunction("greet_python");`
- calling python function by calling `py.greet_python(...)`

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
