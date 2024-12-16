# Tauri Plugin Python

This [tauri](https://v2.tauri.app/) plugin is supposed to make it easy to use Python as backend code.
It uses [PyO3](https://pyo3.rs) to call python from rust.
The plugin reads by default the file `src-tauri/src-python/main.py` during 
compile time and runs it during startup. It then can call functions that were embedded in this python code.

Python code can be registered and called from javascript without the 
requirement to touch rust code at all.
You can still use rust to register all python functions if you have any security concerns,
for example when using inputs from other network interfaces.


You might use this plugin to create simple prototype applications
and later re-write functions in rust to improve
performance, add a specific rust library or just call some 
low-level code.

## Example app

There is a sample Desktop application for Windows/Linux/MacOS using this plugin and vanilla 
Javascript in [examples/plain-javascript](https://github.com/marcomq/tauri-plugin-python/tree/main/examples/plain-javascript)

## Security considerations
This plugin can make it possible to run arbitrary python code that is injected
via Javascript code. It is therefore highly recommended to **not make the user
interface accessible by a network URL**. Otherwise, an XSS vulnerability could 
be used to run random code on the server.

As countermeasuer, the "runPython" command is disabled by default. This function
must not be enabled, once the UI is accessible by network URL.
In addition, the "registerFunction" command cannot be called again once the 
"callFunction" has been called one time. This should prevent re-adding python code, 
once a user has performed any activity in the UI
This is not supposed to be a full protection against remote attacks.

The plugin should only be used in standalone Desktop, MacOS, IOS or Android mode.
