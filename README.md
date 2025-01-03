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

## Security considerations
Generally, this plugin has been created by "security by default" concept. Python functions can onl be called if registered from rust.

Keep in mind that this plugin can make it possible to run arbitrary python code. 
It is therefore highly recommended to **not make the user interface accessible by a network URL**. 

The "runPython" command is disabled by default via permissions. If enabled, it is possible to 
inject python code via javascript.
Also, the function "register" is disabled by default. If enabled, it can 
add control from javascript which functions can be called. This avoids to modify rust code when changing or adding python code.
Both functions can be enabled during development for rapid prototyping.

