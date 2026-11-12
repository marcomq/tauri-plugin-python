# Deploying on macOS (PyO3)

Step-by-step guide to ship a tauri app that uses tauri-plugin-python with the default PyO3 backend and runs on
Macs without Python installed. Using RustPython instead? Then no extra steps are needed; just bundle
`src-python/` (see [Deployment](../README.md#deployment)).

Also see: [Linux](deploy-linux.md) · [Windows](deploy-windows.md)

## 1. Prerequisites

- Node.js 18 or newer (for `npx`), Rust and the tauri prerequisites for macOS.
- A Python on the development machine is only needed for `tauri dev` without a bundled Python.
- Build on the architecture you ship: an Apple Silicon Mac builds `aarch64-apple-darwin`, an Intel Mac
  `x86_64-apple-darwin`. Universal binaries are not supported by the CLI yet.

## 2. Python code and dependencies

```
src-tauri/
├── src-python/
│   ├── main.py             # entry point (or a compiled main.*.so, see README)
│   └── requirements.txt    # pip dependencies, optional
└── tauri.conf.json         # "bundle": { "resources": ["src-python/"] }
```

List pip packages in `src-python/requirements.txt` instead of bundling a `.venv`. A venv only works on the
machine it was created on.

## 3. Bundle Python

Run in the project root or in `src-tauri`:

```sh
npx tauri-plugin-python-cli setup
```

This downloads a relocatable Python to
`src-tauri/pyembed/python`, installs `requirements.txt` into it and changes these files:

| File | Change |
| --- | --- |
| `src-tauri/.cargo/config.toml` | managed block: `PYO3_CONFIG_FILE` and an rpath to `../Resources/pyembed/python/lib` |
| `src-tauri/tauri.macos.conf.json` | the stdlib and `libpython3.13.dylib` as resources |
| `src-tauri/tauri.conf.json` | `bundle.resources` converted to the object form, so tauri merges both files |
| `src-tauri/.gitignore` | `/pyembed/` |
| `src-tauri/src-python/.python-version` | the Python minor version, if the file didn't exist |

Commit the config changes, not `pyembed/`. Run `setup` again after cloning and whenever `requirements.txt`
changes.
Without it, the build fails with `failed to open PyO3 config file at …/pyembed/pyo3-config.txt`,
because the committed `.cargo/config.toml` points to a file in the uncommitted `pyembed/`.

The Python version comes from `src-python/.python-version` (e.g. `3.13`, same format as `uv python pin`). If the
file is missing, `setup` takes the newest Python that reached its x.y.2 bugfix release (`stable`) and writes its
minor version there. Commit that file so the team and CI bundle the same version. To change it, edit the file or
run `setup --python 3.14` (also an exact `3.14.2`, `stable` or `latest`), which updates the file.

`tauri dev` now also uses the bundled Python. Check the project:

```sh
npx tauri-plugin-python-cli doctor
```

## 4. Sign the bundled Python (only for distribution)

Skip this step for local test builds. Apps distributed outside the App Store must be notarized, and
notarization rejects any unsigned native file in the app. Tauri signs only the main binary, not resources, so
sign the Python files **before** `tauri build`, with the same identity tauri uses:

```sh
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
npx tauri-plugin-python-cli sign
```

This signs every Mach-O file in `src-tauri/pyembed/python` with `--options runtime --timestamp`. Pass
`--entitlements <plist>` if your packages need extra hardened-runtime entitlements. Run `sign` again after each
`setup`.

## 5. Build

```sh
npm run tauri build
```

With `APPLE_SIGNING_IDENTITY` and the notarization variables (`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` or
an App Store Connect API key) set, tauri signs and notarizes the app. See the
[tauri macOS signing guide](https://v2.tauri.app/distribute/sign/macos/).

## 6. Verify

```sh
npx tauri-plugin-python-cli doctor
otool -L "src-tauri/target/release/bundle/macos/<App>.app/Contents/MacOS/<binary>" | grep -i python
```

`doctor` checks that the binary loads `@rpath/libpython3.13.dylib`, not a Python from Homebrew or
`/Library/Frameworks`. To test like a user, copy the `.app` to another folder or Mac without Python and start
it. The Python files are in `<App>.app/Contents/Resources/pyembed/python`.

## CI (GitHub Actions)

```yaml
- uses: actions/setup-node@v4
  with:
    node-version: lts/*
- run: npx tauri-plugin-python-cli setup
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}   # avoids GitHub API rate limits
# - run: npx tauri-plugin-python-cli sign       # with APPLE_SIGNING_IDENTITY and the certificate imported
- run: npm install && npm run tauri build
```

`macos-latest` builds for Apple Silicon; use an Intel runner for `x86_64` builds.

## Troubleshooting

| Symptom | Cause / fix |
| --- | --- |
| `Library not loaded: @rpath/libpython3.13.dylib` | `RUSTFLAGS` in the environment replaces the rpaths from `.cargo/config.toml`; unset it or add the rpaths there. `doctor` reports this. |
| App works only on the build Mac | `setup` wasn't run before the build; the binary links the host Python. |
| Notarization: "The binary is not signed" for a file in `Resources/pyembed` | `sign` wasn't run, or `setup` ran again afterwards. |
| `ModuleNotFoundError` for a pip package | add it to `requirements.txt` and run `setup` again. |
