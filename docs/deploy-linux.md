# Deploying on Linux (PyO3)

Step-by-step guide to ship a tauri app that uses tauri-plugin-python with the default PyO3 backend and runs on
Linux systems without a matching Python. Using RustPython instead? Then no extra steps are needed; just bundle
`src-python/` (see [Deployment](../README.md#deployment)).

Also see: [macOS](deploy-macos.md) · [Windows](deploy-windows.md)

## 1. Prerequisites

- Node.js 18 or newer (for `npx`), Rust and the
  [tauri prerequisites for Linux](https://v2.tauri.app/start/prerequisites/#linux).
- `tar` on the `PATH` (to unpack the Python download).
- A Python on the development machine is only needed for `tauri dev` without a bundled Python.
- Build on the oldest distribution you want to support. The app binary links against the build machine's
  glibc and WebKitGTK, so a package built on a new distribution doesn't start on an older one. Ubuntu 22.04 is a
  common baseline.

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
| `src-tauri/.cargo/config.toml` | managed block: `PYO3_CONFIG_FILE` and an rpath to `$ORIGIN/../lib/<productName>/pyembed/python/lib` |
| `src-tauri/tauri.linux.conf.json` | the stdlib and `libpython3.13.so.1.0` as resources |
| `src-tauri/tauri.conf.json` | `bundle.resources` converted to the object form, so tauri merges both files |
| `src-tauri/.gitignore` | `/pyembed/` |
| `src-tauri/src-python/.python-version` | the Python minor version, if the file didn't exist |

The rpath contains the `productName` from `tauri.conf.json`. Run `setup` again when you rename the app.

Commit the config changes, not `pyembed/`. Run `setup` again after cloning and whenever `requirements.txt`
changes.
Without it, the build fails with `failed to open PyO3 config file at …/pyembed/pyo3-config.txt`,
because the committed `.cargo/config.toml` points to a file in the uncommitted `pyembed/`.

The Python version comes from `src-python/.python-version` (e.g. `3.13`, same format as `uv python pin`). If the
file is missing, `setup` takes the newest Python that reached its x.y.2 bugfix release (`stable`) and writes its
minor version there. Commit that file so the team and CI bundle the same version. To change it, edit the file or
run `npx tauri-plugin-python-cli setup --python 3.14` (also an exact `3.14.2`, `stable` or `latest`), which updates the file.

`tauri dev` now also uses the bundled Python. Check the project:

```sh
npx tauri-plugin-python-cli doctor
```

## 4. Build

```sh
npm run tauri build                     # all configured bundles
npm run tauri build -- --bundles deb    # or only one: deb, rpm, appimage
```

## 5. Verify

```sh
npx tauri-plugin-python-cli doctor
readelf -d src-tauri/target/release/<binary> | grep -E 'NEEDED.*python|RUNPATH|RPATH'
```

`doctor` checks that the binary needs `libpython3.13.so.1.0` and has the rpath into the app's lib folder. To
test like a user, install the package on a clean machine or container without Python:

```sh
dpkg -c src-tauri/target/release/bundle/deb/*.deb | grep pyembed   # files land in /usr/lib/<productName>/pyembed
```

The CI of this repository installs and starts the `.deb` this way. `rpm` and AppImage use the same layout but
are not tested in CI.

## CI (GitHub Actions)

```yaml
runs-on: ubuntu-22.04
steps:
  - uses: actions/checkout@v4
  - run: |
      sudo apt-get update
      sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
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
| `error while loading shared libraries: libpython3.13.so.1.0` | `RUSTFLAGS` in the environment replaces the rpaths from `.cargo/config.toml`, or `productName` changed after `setup`. `doctor` reports both. |
| App works only on the build machine | `setup` wasn't run before the build; the binary links the host Python. |
| App doesn't start on an older distribution (`GLIBC_2.xx not found`) | build on an older distribution, see Prerequisites. |
| `ModuleNotFoundError` for a pip package | add it to `requirements.txt` and run `setup` again. |
