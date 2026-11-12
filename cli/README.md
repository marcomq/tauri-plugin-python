# tauri-plugin-python-cli

Deployment helper for [tauri-plugin-python](https://github.com/marcomq/tauri-plugin-python).

With the PyO3 backend, the app links against the `libpython` found at build
time and only starts on machines that have exactly that Python at exactly that
location. `setup` bundles a relocatable
[python-build-standalone](https://github.com/astral-sh/python-build-standalone)
interpreter instead, so the app runs everywhere, in `tauri dev` and in the
installed bundle.

```sh
npx tauri-plugin-python-cli setup     # bundle Python + src-python/requirements.txt
npx tauri-plugin-python-cli doctor    # check the project (and built binaries)
npx tauri-plugin-python-cli sign      # macOS: codesign the bundled Python, before `tauri build`
```

Step-by-step guides: [macOS](https://github.com/marcomq/tauri-plugin-python/blob/main/docs/deploy-macos.md) ·
[Linux](https://github.com/marcomq/tauri-plugin-python/blob/main/docs/deploy-linux.md) ·
[Windows](https://github.com/marcomq/tauri-plugin-python/blob/main/docs/deploy-windows.md)

Run the commands in the project root or in `src-tauri`.

## setup

- downloads python-build-standalone for this machine into `src-tauri/pyembed/python`
  (checksum verified, cached in `~/.cache/tauri-plugin-python-cli`)
- removes test suite, tkinter and other parts an app doesn't need
- installs `src-python/requirements.txt` into it with pip
- writes `pyembed/pyo3-config.txt` and a managed block in `.cargo/config.toml`
  (`PYO3_CONFIG_FILE` and the rpaths to find `libpython`)
- registers the runtime files as resources in `tauri.<os>.conf.json` and
  converts `bundle.resources` in `tauri.conf.json` to the object form, so tauri
  can merge both
- adds `/pyembed/` to `src-tauri/.gitignore`

`pyembed/` is machine specific and not committed: run `setup` again after
cloning, in CI before `tauri build`, and whenever requirements change. Otherwise
the build fails with `failed to open PyO3 config file at …/pyembed/pyo3-config.txt`.

Python version: `src-python/.python-version` (e.g. `3.13`, as written by
`uv python pin`). If it is missing, `setup` takes `stable`, the newest Python that
reached its x.y.2 bugfix release, and writes its minor version there; commit the
file. `--python` overrides it and updates the file; `latest` takes the newest one.

Options: `--python 3.13` (or an exact `3.13.5`, `stable`, `latest`), `--release <tag>`,
`--requirements <file>`, `--force` (download again).

## doctor

Checks entry point, resources, bundled Python, whether `src-python/.venv` matches
it, `PYO3_CONFIG_FILE`, rpaths,
`RUSTFLAGS` overrides and, if already built, whether the binary links the
bundled `libpython` and not the host one. For the RustPython backend it
detects the duplicate `malachite-bigint` versions that break the build;
`doctor --fix` pins them in `Cargo.lock`.

## sign (macOS)

Notarization rejects unsigned native files anywhere in the app, and tauri only
signs the main binary and frameworks. `sign` signs every Mach-O file in the
bundled Python with `--options runtime --timestamp`, using `--identity` or
`APPLE_SIGNING_IDENTITY` (the same variable `tauri build` uses). It signs the
files in `src-tauri/pyembed`, so run it after `setup` and before `tauri build`.
