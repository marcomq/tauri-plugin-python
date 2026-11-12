// @ts-check
// `doctor`: checks the project for the usual deployment problems.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { cargoConfigConflicts, hasManagedBlock, resourcesAsObject } from "./configs.js";
import { isVersionAlias } from "./pbs.js";
import { MANIFEST, PYO3_CONFIG, PYTHON_DIR, hostTriple, layout, osOf } from "./platform.js";
import {
  duplicateLockVersions,
  findSrcTauri,
  hasCompiledEntry,
  packageName,
  pinnedPythonVersion,
  pluginBackend,
  readJson,
  readText,
  venvMismatch,
} from "./project.js";

/** @typedef {import("./platform.js").Os} Os */
/** @typedef {import("./setup.js").Manifest} Manifest */

/**
 * @typedef {object} Finding
 * @property {"ok" | "warn" | "error"} level
 * @property {string} msg
 */

/**
 * @typedef {object} Report
 * @property {(msg: string) => void} ok
 * @property {(msg: string) => void} warn
 * @property {(msg: string) => void} error
 */

/**
 * @typedef {object} DoctorOptions
 * @property {string} [project]
 * @property {boolean} [fix]  apply the safe automatic fixes
 */

const MALACHITE = "malachite-bigint";

/**
 * @param {DoctorOptions} opts
 * @returns {Finding[]}
 */
export function doctor(opts) {
  /** @type {Finding[]} */
  const findings = [];
  /** @type {Report} */
  const report = {
    ok: (msg) => findings.push({ level: "ok", msg }),
    warn: (msg) => findings.push({ level: "warn", msg }),
    error: (msg) => findings.push({ level: "error", msg }),
  };

  const srcTauri = findSrcTauri(opts.project);
  const backend = pluginBackend(readText(path.join(srcTauri, "Cargo.toml")));
  if (!backend) {
    report.error("tauri-plugin-python is not a dependency in src-tauri/Cargo.toml");
    return findings;
  }
  report.ok(`Backend: ${backend}`);

  const srcPython = path.join(srcTauri, "src-python");
  if (fs.existsSync(path.join(srcPython, "main.py"))) report.ok("Entry point: src-python/main.py");
  else if (hasCompiledEntry(srcPython)) report.ok("Entry point: compiled 'main' module (Nuitka/Cython)");
  else report.error("No src-python/main.py (or compiled main.*.so / main.*.pyd / main.pyc) found");

  const os = osOf(hostTriple());
  const resources = mergedResources(srcTauri, os);
  if (Object.keys(resources).some((src) => /^(\.\/)?src-python\b/.test(src))) {
    report.ok("src-python is bundled as resource");
  } else {
    report.error("src-python/ is missing in bundle.resources of tauri.conf.json - release builds won't find main.py");
  }
  if (fs.existsSync(path.join(srcPython, ".venv"))) {
    report.warn("src-python/.venv only works on the build machine; bundle dependencies with 'setup' (PyO3) instead");
  }

  if (backend === "rustpython") checkRustPython(srcTauri, Boolean(opts.fix), report);
  else checkPyo3(srcTauri, os, resources, report);
  return findings;
}

/**
 * Resources after tauri merged `tauri.<os>.conf.json` into `tauri.conf.json`.
 * @param {string} srcTauri
 * @param {Os} os
 * @returns {Record<string, string>}
 */
function mergedResources(srcTauri, os) {
  const base = readJson(path.join(srcTauri, "tauri.conf.json"));
  const platformRes = readJson(path.join(srcTauri, `tauri.${os}.conf.json`))?.bundle?.resources;
  const baseRes = base?.bundle?.resources;
  // JSON Merge Patch replaces anything that isn't an object on both sides.
  if (!platformRes) return resourcesAsObject(baseRes);
  if (Array.isArray(platformRes) || Array.isArray(baseRes)) return resourcesAsObject(platformRes);
  return { ...baseRes, ...platformRes };
}

/**
 * @param {string} srcTauri
 * @param {boolean} fix
 * @param {Report} report
 */
function checkRustPython(srcTauri, fix, report) {
  const versions = duplicateLockVersions(readText(path.join(srcTauri, "Cargo.lock")), MALACHITE);
  if (!versions.length) {
    report.ok(`Cargo.lock: single ${MALACHITE} version`);
    return;
  }
  const [oldest, ...newer] = versions.sort(compareVersions);
  if (!fix) {
    report.error(`Cargo.lock has ${MALACHITE} ${versions.join(" and ")}; RustPython fails to compile. Run 'doctor --fix'`);
    return;
  }
  for (const v of newer) {
    const args = ["update", "-p", `${MALACHITE}@${v}`, "--precise", oldest];
    const res = spawnSync("cargo", args, { cwd: srcTauri, stdio: "inherit" });
    if (res.status !== 0) {
      report.error(`cargo ${args.join(" ")} failed`);
      return;
    }
  }
  report.ok(`Fixed: pinned ${MALACHITE} to ${oldest} in Cargo.lock`);
}

/**
 * @param {string} srcTauri
 * @param {Os} os
 * @param {Record<string, string>} resources
 * @param {Report} report
 */
function checkPyo3(srcTauri, os, resources, report) {
  /** @type {Manifest | undefined} */
  const manifest = readJson(path.join(srcTauri, MANIFEST));
  if (!manifest) {
    report.warn(
      "No bundled Python: the app links the Python found at build time and only starts where that exact version is installed. Run 'setup' to bundle one",
    );
    return;
  }
  report.ok(`Bundled Python ${manifest.python} (${manifest.triple})`);
  const pin = pinnedPythonVersion(srcTauri);
  if (pin && !isVersionAlias(pin) && !`${manifest.python}.`.startsWith(`${pin}.`)) {
    report.error(`src-python/.python-version wants ${pin}, the bundled Python is ${manifest.python}. Run 'setup'`);
  }
  const mismatch = venvMismatch(srcTauri, manifest.python);
  if (mismatch) report.warn(mismatch);
  if (manifest.triple !== hostTriple()) {
    report.error(`Bundled Python is for ${manifest.triple}, this machine is ${hostTriple()}. Run 'setup --force'`);
  }
  const l = layout(os, manifest.python);
  if (!fs.existsSync(path.join(srcTauri, l.stdlib))) report.error(`${l.stdlib} is missing. Run 'setup --force'`);

  const libDir = /^lib_dir=(.*)$/m.exec(readText(path.join(srcTauri, PYO3_CONFIG)) ?? "")?.[1];
  if (!libDir) report.error(`${PYO3_CONFIG} is missing. Run 'setup'`);
  else if (path.resolve(libDir) !== path.resolve(srcTauri, l.libDir)) {
    report.error(`${PYO3_CONFIG} points to ${libDir} (project moved?). Run 'setup'`);
  } else report.ok(`${PYO3_CONFIG} is up to date`);

  const cargoConfig = readText(path.join(srcTauri, ".cargo", "config.toml"));
  if (!cargoConfig || !hasManagedBlock(cargoConfig)) {
    report.error(".cargo/config.toml lacks the PYO3_CONFIG_FILE/rpath settings. Run 'setup'");
  } else if (cargoConfigConflicts(cargoConfig).length) {
    report.warn(".cargo/config.toml has own [env]/rustflags settings; check that they don't override the managed block");
  } else report.ok(".cargo/config.toml sets PYO3_CONFIG_FILE and rpaths");
  for (const v of ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"]) {
    if (process.env[v]) report.warn(`${v} is set and replaces the rpaths from .cargo/config.toml`);
  }
  if (process.env.PYO3_PYTHON) report.warn("PYO3_PYTHON is set; PYO3_CONFIG_FILE takes precedence");

  const missing = Object.keys(l.resources).filter(
    (src) => fs.existsSync(path.join(srcTauri, src)) && !(src in resources),
  );
  if (missing.length) {
    report.error(`Bundled Python not in tauri.${os}.conf.json resources (${missing.join(", ")}). Run 'setup'`);
  } else report.ok(`Bundled Python is registered in tauri.${os}.conf.json`);

  checkBinaries(srcTauri, os, manifest, report);
}

/**
 * Inspects already built binaries for the classic "links the host Python" mistake.
 * @param {string} srcTauri
 * @param {Os} os
 * @param {Manifest} manifest
 * @param {Report} report
 */
function checkBinaries(srcTauri, os, manifest, report) {
  const name = packageName(readText(path.join(srcTauri, "Cargo.toml")));
  if (!name) return;
  const exe = os === "windows" ? `${name}.exe` : name;
  for (const profile of ["release", "debug"]) {
    const bin = path.join(srcTauri, "target", profile, exe);
    if (!fs.existsSync(bin)) continue;
    const rel = path.relative(srcTauri, bin);
    if (os === "windows") {
      const dll = Object.values(layout(os, manifest.python).resources).find((t) => t.endsWith(".dll"));
      if (dll && fs.existsSync(path.join(path.dirname(bin), dll))) report.ok(`${rel}: ${dll} next to the exe`);
      else report.warn(`${rel}: ${dll} is not next to the exe; rebuild after 'setup'`);
      continue;
    }
    const linkage = inspectLinkage(bin, os);
    if (!linkage) continue;
    const bundled = linkage.rpaths.some((p) => p.includes(`${PYTHON_DIR}/lib`));
    if (!linkage.libpython) report.warn(`${rel}: no libpython dependency found`);
    else if (linkage.libpython.startsWith("/")) {
      report.error(`${rel}: links the host Python (${linkage.libpython}); rebuild after 'setup'`);
    } else if (!bundled) report.error(`${rel}: no rpath to ${PYTHON_DIR}/lib; rebuild after 'setup'`);
    else report.ok(`${rel}: links ${linkage.libpython} from the bundled Python`);
  }
}

/**
 * libpython dependency and rpaths of a binary (otool on macOS, readelf on Linux).
 * @param {string} bin
 * @param {Os} os
 * @returns {{ libpython: string | undefined, rpaths: string[] } | undefined}
 */
function inspectLinkage(bin, os) {
  const cmds = os === "macos" ? [["otool", "-L"], ["otool", "-l"]] : [["readelf", "-d"]];
  const out = [];
  for (const [cmd, flag] of cmds) {
    const res = spawnSync(cmd, [flag, bin], { encoding: "utf8" });
    if (res.status !== 0) return undefined;
    out.push(res.stdout);
  }
  const text = out.join("\n");
  const libpython =
    os === "macos"
      ? /^\s*(\S*(?:libpython|Python\.framework\/)\S*)/m.exec(out[0])?.[1]
      : /NEEDED.*\[(libpython[^\]]*)\]/.exec(text)?.[1];
  const rpaths =
    os === "macos"
      ? [...text.matchAll(/^\s*path (\S+) \(offset/gm)].map((m) => m[1])
      : [...text.matchAll(/(?:RUNPATH|RPATH).*\[([^\]]*)\]/g)].flatMap((m) => m[1].split(":"));
  return { libpython, rpaths };
}

/**
 * @param {string} a
 * @param {string} b
 */
function compareVersions(a, b) {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    if ((pa[i] ?? 0) !== (pb[i] ?? 0)) return (pa[i] ?? 0) - (pb[i] ?? 0);
  }
  return 0;
}
