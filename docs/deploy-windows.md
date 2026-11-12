# Deploying on Windows (PyO3)

Step-by-step guide to ship a tauri app that uses tauri-plugin-python with the default PyO3 backend and runs on
Windows machines without Python installed. Using RustPython instead? Then no extra steps are needed; just bundle
`src-python/` (see [Deployment](../README.md#deployment)).

Also see: [macOS](deploy-macos.md) · [Linux](deploy-linux.md)

## 1. Prerequisites

- Node.js 18 or newer (for `npx`), Rust with the MSVC toolchain and the
  [tauri prerequisites for Windows](https://v2.tauri.app/start/prerequisites/#windows).
- `tar` on the `PATH`. It ships with Windows 10 and newer.
- A Python on the development machine is only needed for `tauri dev` without a bundled Python.
- Build on the architecture you ship (`x86_64` or `aarch64`).

## 2. Python code and dependencies

```
src-tauri\
├── src-python\
│   ├── main.py             # entry point (or a compiled main.*.pyd, see README)
│   └── requirements.txt    # pip dependencies, optional
└── tauri.conf.json         # "bundle": { "resources": ["src-python/"] }
```

List pip packages in `src-python\requirements.txt` instead of bundling a `.venv`. A venv only works on the
machine it was created on.

## 3. Bundle Python

Run in the project root or in `src-tauri`, in PowerShell, cmd or Git Bash:

```sh
npx tauri-plugin-python-cli setup
```

This downloads a relocatable Python to
`src-tauri\pyembed\python`, installs `requirements.txt` into it and changes these files:

| File | Change |
| --- | --- |
| `src-tauri\.cargo\config.toml` | managed block: `PYO3_CONFIG_FILE` (link against `pyembed\python\libs\python313.lib`) |
| `src-tauri\tauri.windows.conf.json` | the Python folder as resource, plus `python313.dll` and `python3.dll` next to the `.exe` |
| `src-tauri\tauri.conf.json` | `bundle.resources` converted to the object form, so tauri merges both files |
| `src-tauri\.gitignore` | `/pyembed/` |
| `src-tauri\src-python\.python-version` | the Python minor version, if the file didn't exist |

Windows has no rpath: the loader finds `python313.dll` because it is placed next to the `.exe`.

Commit the config changes, not `pyembed\`. Run `setup` again after cloning and whenever `requirements.txt`
changes.
Without it, the build fails with `failed to open PyO3 config file at …\pyembed\pyo3-config.txt`,
because the committed `.cargo/config.toml` points to a file in the uncommitted `pyembed/`.

The Python version comes from `src-python/.python-version` (e.g. `3.13`, same format as `uv python pin`). If the
file is missing, `setup` takes the newest Python that reached its x.y.2 bugfix release (`stable`) and writes its
minor version there. Commit that file so the team and CI bundle the same version. To change it, edit the file or
run `setup --python 3.14` (also an exact `3.14.2`, `stable` or `latest`), which updates the file.

`tauri dev` now also uses the bundled Python. Check the project:

```sh
npx tauri-plugin-python-cli doctor
```

## 4. Build

```sh
npm run tauri build                      # all configured bundles
npm run tauri build -- --bundles nsis    # or only one: nsis, msi
```

To sign the app, see the [tauri Windows signing guide](https://v2.tauri.app/distribute/sign/windows/). Tauri
signs the `.exe` and the installer, not the bundled Python files; Windows usually doesn't require them to be
signed.

## 5. Verify

```sh
npx tauri-plugin-python-cli doctor
```

`doctor` checks that `python313.dll` sits next to the built `.exe`. To test like a user, install the app on a
machine or VM without Python and start it. After installation, the install folder contains the `.exe`,
`python313.dll`, `python3.dll`, `pyembed\python\` and `src-python\`.

The CI of this repository installs the NSIS installer silently and starts the app this way. MSI uses the same
layout but is not tested in CI.

## CI (GitHub Actions)

```yaml
runs-on: windows-latest
steps:
  - uses: actions/setup-node@v4
    with:
      node-version: lts/*
  - run: npx tauri-plugin-python-cli setup
    env:
      GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}   # avoids GitHub API rate limits
  - run: npm install && npm run tauri build
```

## Troubleshooting

| Symptom | Cause / fix |
| --- | --- |
| `python313.dll was not found` | the DLL resources are missing in `tauri.windows.conf.json`; run `setup` again and check with `doctor`. |
| App works only on the build machine | `setup` wasn't run before the build; the binary links the host Python. |
| `print()` output is missing in release builds | release builds have no console; see [Windows: hidden console & stdio](../README.md#windows-hidden-console--stdio). |
| `ModuleNotFoundError` for a pip package | add it to `requirements.txt` and run `setup` again. |
