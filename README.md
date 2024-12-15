# Tauri Plugin python

This plugin is supposed to make it easy to use Python as backend code.

Python code can be registered and called from javascript without the 
requirement to touch rust code at all.

## Example

There is a running sample application using this plugin and vanilla 
Javascript in https://github.com/marcomq/tauri-python-plugin/tree/main/examples/plain-javascript

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

The plugin should only be used in Desktop, MacOS, OSX or Android mode.