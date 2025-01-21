# Tauri + Python Plugin + Vanilla

This template should help get you started developing with Tauri and Python Plugin 
and Vanilla Javascript

## Quick Start
- install rust, python and nodeJs
- run `npm install` in the tauri-plugin-python base path (`cd ../../`)
- run `npm run build` in the tauri-plugin-python base path
- run `npm install` in the example path (`cd examples/plain-javascript`)
- run `npm run tauri dev` to start the application

To run this sample app on iOS:
- run `npx @tauri-apps/cli plugin ios init` to init ios project files
- run `npm run tauri ios dev` to start the application on iOS in develop mode

## Manual modifications on default template to add plugin:
- add `tauri-plugin-python` to Cargo.toml 
- add `tauri-plugin-python-api` to package.json
- modify `permissions:[]` in src-tauri/capabilities/default.json and add "python:default"  
- modify `src-tauri/src-python/main.py` and add python code, for example `def greet_python(..`
- add `.plugin(tauri_plugin_python::init(["greet_python"]))` to `src-tauri/src/lib.rs`
- include javascript for python plugin in the index.html file for example by adding `<script type="module" src="/tauri-plugin-python-api/index.iife.js" defer></script>`
- register python functions in javascript by calling `registerJs("greet_python");`
- calling python function by calling `call.greet_python(...)`

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
