// @ts-check
// Platform specific layout of a python-build-standalone install and how it is
// linked and bundled. Everything here is pure, so it can be unit tested for
// all platforms on any host.

import path from "node:path";

/** @typedef {"macos" | "linux" | "windows"} Os */

/**
 * @typedef {object} Version
 * @property {number} major
 * @property {number} minor
 * @property {number | undefined} patch
 */

/**
 * @typedef {object} Layout
 * @property {string} python    interpreter, used for pip
 * @property {string} libDir    directory with the import library / libpython
 * @property {string} libName   library name for the linker
 * @property {string} stdlib    stdlib directory, used to validate the install
 * @property {string[]} prune   unneeded parts, relative to the python dir (`*` allowed)
 * @property {Record<string, string>} resources  tauri resources: source -> target
 * @property {Record<string, string>} copies     files copied by setup: source -> copy
 */

export const EMBED_DIR = "pyembed";
export const PYTHON_DIR = `${EMBED_DIR}/python`;
export const PYO3_CONFIG = `${EMBED_DIR}/pyo3-config.txt`;
export const MANIFEST = `${EMBED_DIR}/tauri-plugin-python.json`;

/** @type {Record<string, string>} */
const TRIPLES = {
  "darwin-arm64": "aarch64-apple-darwin",
  "darwin-x64": "x86_64-apple-darwin",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "win32-x64": "x86_64-pc-windows-msvc",
  "win32-arm64": "aarch64-pc-windows-msvc",
};

/**
 * @param {string} [platform]
 * @param {string} [arch]
 * @returns {string}
 */
export function hostTriple(platform = process.platform, arch = process.arch) {
  const triple = TRIPLES[`${platform}-${arch}`];
  if (!triple) {
    throw new Error(`No standalone Python available for ${platform}/${arch}`);
  }
  return triple;
}

/**
 * @param {string} triple
 * @returns {Os}
 */
export function osOf(triple) {
  if (triple.includes("apple-darwin")) return "macos";
  if (triple.includes("windows")) return "windows";
  return "linux";
}

/**
 * Splits "3.13" or "3.13.18" into numbers.
 * @param {string} version
 * @returns {Version}
 */
export function parseVersion(version) {
  const m = /^(\d+)\.(\d+)(?:\.(\d+))?$/.exec(String(version).trim());
  if (!m) throw new Error(`Invalid Python version '${version}', expected e.g. 3.13`);
  return { major: +m[1], minor: +m[2], patch: m[3] === undefined ? undefined : +m[3] };
}

/**
 * Files of the Python install relative to the project's `src-tauri` dir.
 * Only what the interpreter needs at runtime is bundled (no `bin/`, headers
 * or pkgconfig).
 * @param {Os} os
 * @param {string} version
 * @returns {Layout}
 */
export function layout(os, version) {
  const { major, minor } = parseVersion(version);
  const xy = `${major}.${minor}`;
  if (os === "windows") {
    const dll = `python${major}${minor}.dll`;
    return {
      python: `${PYTHON_DIR}/python.exe`,
      libDir: `${PYTHON_DIR}/libs`,
      libName: `python${major}${minor}`,
      stdlib: `${PYTHON_DIR}/Lib`,
      prune: ["Lib/test", "Lib/idlelib", "Lib/tkinter", "Lib/turtledemo", "tcl", "DLLs/_tkinter.pyd"],
      resources: {
        [`${PYTHON_DIR}/`]: `${PYTHON_DIR}/`,
        [`${EMBED_DIR}/${dll}`]: dll,
        [`${EMBED_DIR}/python${major}.dll`]: `python${major}.dll`,
      },
      // The NSIS bundler takes each source file once, and the python dir already contains the DLLs.
      copies: {
        [`${PYTHON_DIR}/${dll}`]: `${EMBED_DIR}/${dll}`,
        [`${PYTHON_DIR}/python${major}.dll`]: `${EMBED_DIR}/python${major}.dll`,
      },
    };
  }
  const libpython =
    os === "macos" ? `libpython${xy}.dylib` : `libpython${xy}.so.1.0`;
  return {
    python: `${PYTHON_DIR}/bin/python${xy}`,
    libDir: `${PYTHON_DIR}/lib`,
    libName: `python${xy}`,
    stdlib: `${PYTHON_DIR}/lib/python${xy}`,
    prune: [
      `lib/python${xy}/test`,
      `lib/python${xy}/idlelib`,
      `lib/python${xy}/tkinter`,
      `lib/python${xy}/turtledemo`,
      `lib/python${xy}/lib-dynload/_tkinter*`,
      "lib/tcl*",
      "lib/tk*",
      "lib/itcl*",
      "lib/thread*",
      "lib/libtcl*",
      "lib/libtk*",
      "share",
    ],
    resources: {
      [`${PYTHON_DIR}/lib/python${xy}/`]: `${PYTHON_DIR}/lib/python${xy}/`,
      [`${PYTHON_DIR}/lib/${libpython}`]: `${PYTHON_DIR}/lib/${libpython}`,
    },
    copies: {},
  };
}

/**
 * Linker rpaths so the app finds the bundled libpython, both for
 * `target/<profile>` (tauri copies resources next to the binary) and for the
 * installed bundle. Windows finds the DLL next to the exe instead.
 * @param {Os} os
 * @param {string | undefined} productName
 * @returns {string[]}
 */
export function rpaths(os, productName) {
  const lib = `${PYTHON_DIR}/lib`;
  if (os === "macos") {
    return [`@executable_path/../Resources/${lib}`, `@executable_path/${lib}`];
  }
  if (os === "linux") {
    return [`$ORIGIN/../lib/${productName}/${lib}`, `$ORIGIN/${lib}`];
  }
  return [];
}

/**
 * Contents of `PYO3_CONFIG_FILE`. The python-build-standalone sysconfig points
 * to its build prefix, so PyO3 can't derive the library location itself.
 * @param {string} srcTauri
 * @param {Os} os
 * @param {string} version
 * @returns {string}
 */
export function pyo3Config(srcTauri, os, version) {
  const l = layout(os, version);
  const { major, minor } = parseVersion(version);
  return [
    "implementation=CPython",
    `version=${major}.${minor}`,
    "shared=true",
    "abi3=false",
    `lib_name=${l.libName}`,
    `lib_dir=${path.resolve(srcTauri, l.libDir)}`,
    `executable=${path.resolve(srcTauri, l.python)}`,
    "pointer_width=64",
    "build_flags=",
    "suppress_build_script_link_lines=false",
    "",
  ].join("\n");
}
