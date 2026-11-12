# CI replacement for src-python/main.py: reports which Python the bundled app
# runs and whether stdlib and native modules load, to the file in E2E_REPORT.
import os
import sys

_tauri_plugin_functions = ["greet_python"]

lines = ["prefix=" + sys.prefix, "version=" + sys.version.split()[0]]
for mod in ["json", "asyncio", "ssl", "sqlite3", "ctypes", "decimal", "zlib", "lzma", "hashlib", "markupsafe._speedups"]:
    try:
        __import__(mod)
        lines.append("ok " + mod)
    except Exception as e:
        lines.append("FAIL %s: %r" % (mod, e))
with open(os.environ["E2E_REPORT"], "w") as f:
    f.write("\n".join(lines) + "\n")


def greet_python(input):
    return "Hello " + str(input)
