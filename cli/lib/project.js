// @ts-check
// Locating the tauri project and reading the parts of its config we need.

import fs from "node:fs";
import path from "node:path";

/** @typedef {"pyo3" | "rustpython"} Backend */

/**
 * Returns the `src-tauri` dir for a project root or a `src-tauri` dir.
 * @param {string} [start]
 * @returns {string}
 */
export function findSrcTauri(start = process.cwd()) {
  for (const dir of [start, path.join(start, "src-tauri")]) {
    if (fs.existsSync(path.join(dir, "tauri.conf.json"))) return path.resolve(dir);
  }
  if (fs.existsSync(path.join(start, "src-tauri"))) {
    throw new Error(
      "Only tauri.conf.json is supported (no tauri.conf.json5 / Tauri.toml) - see the deployment guide for manual steps",
    );
  }
  throw new Error(`No tauri project found in ${start} (expected src-tauri/tauri.conf.json)`);
}

/**
 * @param {string} file
 * @returns {any}
 */
export function readJson(file) {
  return fs.existsSync(file) ? JSON.parse(fs.readFileSync(file, "utf8")) : undefined;
}

/**
 * @param {string} file
 * @param {unknown} value
 */
export function writeJson(file, value) {
  fs.writeFileSync(file, JSON.stringify(value, null, 2) + "\n");
}

/**
 * @param {string} file
 * @returns {string | undefined}
 */
export function readText(file) {
  return fs.existsSync(file) ? fs.readFileSync(file, "utf8") : undefined;
}

/**
 * Same fallback as tauri: `productName`, then the cargo package name.
 * @param {{ productName?: string } | undefined} tauriConf
 * @param {string | undefined} cargoToml
 * @returns {string | undefined}
 */
export function productName(tauriConf, cargoToml) {
  return tauriConf?.productName ?? packageName(cargoToml);
}

/**
 * @param {string | undefined} cargoToml
 * @returns {string | undefined}
 */
export function packageName(cargoToml) {
  const section = /^\[package\][^\n]*\n([\s\S]*?)(?=^\s*\[|(?![\s\S]))/m.exec(cargoToml ?? "")?.[1];
  return /^\s*name\s*=\s*"([^"]+)"/m.exec(section ?? "")?.[1];
}

/**
 * Backend selected for tauri-plugin-python in `Cargo.toml`, or undefined if
 * the plugin is not a dependency.
 * @param {string | undefined} cargoToml
 * @returns {Backend | undefined}
 */
export function pluginBackend(cargoToml) {
  const text = cargoToml ?? "";
  const spec =
    /^\s*tauri-plugin-python\s*=\s*(.*)$/m.exec(text)?.[1] ??
    /^\[dependencies\.tauri-plugin-python\][^\n]*\n((?:(?!\s*\[).*(?:\n|$))*)/m.exec(text)?.[1];
  if (spec === undefined) return undefined;
  const features = /features\s*=\s*\[([^\]]*)\]/.exec(spec.replace(/#.*$/gm, ""))?.[1] ?? "";
  return /["']rustpython["']/.test(features) ? "rustpython" : "pyo3";
}

/**
 * Whether `dir` contains a compiled `main` module (Nuitka / Cython).
 * @param {string} dir
 */
export function hasCompiledEntry(dir) {
  if (!fs.existsSync(dir)) return false;
  return fs.readdirSync(dir).some((f) => /^main\.(.*\.)?(so|pyd|pyc)$/.test(f));
}

/**
 * Versions of lockfile package `name` if it appears more than once.
 * @param {string | undefined} cargoLock
 * @param {string} name
 * @returns {string[]}
 */
export function duplicateLockVersions(cargoLock, name) {
  const versions = [];
  const re = /\[\[package\]\]\s*\nname\s*=\s*"([^"]+)"\s*\nversion\s*=\s*"([^"]+)"/g;
  for (const m of (cargoLock ?? "").matchAll(re)) {
    if (m[1] === name) versions.push(m[2]);
  }
  return versions.length > 1 ? versions : [];
}

/**
 * Python version ("3.13") a venv was created with, from its `pyvenv.cfg`.
 * @param {string} venvDir
 * @returns {string | undefined}
 */
export function venvPythonVersion(venvDir) {
  const cfg = readText(path.join(venvDir, "pyvenv.cfg")) ?? "";
  const m = /^\s*(?:version_info|version)\s*=\s*(\d+)\.(\d+)/m.exec(cfg);
  return m ? `${m[1]}.${m[2]}` : undefined;
}

/**
 * Warning if `src-python/.venv` doesn't match the bundled Python and will
 * therefore be ignored by the plugin at runtime.
 * @param {string} srcTauri
 * @param {string} bundledVersion  e.g. "3.13.18"
 * @returns {string | undefined}
 */
export function venvMismatch(srcTauri, bundledVersion) {
  const venv = path.join(srcTauri, "src-python", ".venv");
  const venvVersion = venvPythonVersion(venv);
  const bundled = bundledVersion.split(".").slice(0, 2).join(".");
  if (!venvVersion || venvVersion === bundled) return undefined;
  return (
    `src-python/.venv was created with Python ${venvVersion}, the bundled Python is ${bundled}: ` +
    `the app ignores it at runtime. Put its packages into src-python/requirements.txt and remove it, ` +
    `or recreate it with Python ${bundled}`
  );
}

/**
 * Python version pinned in `src-python/.python-version` (pyenv/uv format).
 * @param {string} srcTauri
 * @returns {string | undefined}
 */
export function pinnedPythonVersion(srcTauri) {
  const text = readText(path.join(srcTauri, "src-python", ".python-version")) ?? "";
  return text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .find((l) => l && !l.startsWith("#"));
}

/**
 * Writes `src-python/.python-version` if `src-python/` exists.
 * @param {string} srcTauri
 * @param {string} version
 * @returns {string | undefined}  the written file
 */
export function writePythonPin(srcTauri, version) {
  const dir = path.join(srcTauri, "src-python");
  if (!fs.existsSync(dir)) return undefined;
  const file = path.join(dir, ".python-version");
  if (pinnedPythonVersion(srcTauri) === version) return undefined;
  fs.writeFileSync(file, `${version}\n`);
  return file;
}
