// @ts-check
// `setup`: bundle a standalone Python for the PyO3 backend.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import {
  cargoConfigBlock,
  cargoConfigConflicts,
  resourcesAsObject,
  unconvertibleResources,
  upsertCargoConfig,
  withGitignoreEntry,
  withPythonResources,
} from "./configs.js";
import { bundlesSrcPython, mergedResources } from "./doctor.js";
import { download, extract, fetchRelease, isVersionAlias, selectAsset } from "./pbs.js";
import {
  EMBED_DIR,
  MANIFEST,
  PYO3_CONFIG,
  PYTHON_DIR,
  hostTriple,
  layout,
  osOf,
  pyo3Config,
} from "./platform.js";
import {
  findSrcTauri,
  pinnedPythonVersion,
  pluginBackend,
  productName,
  readJson,
  readText,
  venvMismatch,
  writeJson,
  writePythonPin,
} from "./project.js";

/**
 * @typedef {object} SetupOptions
 * @property {string} [project]
 * @property {string} [python]        e.g. "3.13", "3.13.5", "stable" or "latest"
 * @property {string} [release]       python-build-standalone release tag
 * @property {string} [requirements]
 * @property {boolean} [force]
 */

/**
 * Written to `pyembed/tauri-plugin-python.json`.
 * @typedef {object} Manifest
 * @property {string} python
 * @property {string} release
 * @property {string} triple
 * @property {string} asset
 */

export const DEFAULT_PYTHON = "stable";

/**
 * @param {SetupOptions} opts
 * @param {(msg: string) => void} [log]
 */
export async function setup(opts, log = console.log) {
  const srcTauri = findSrcTauri(opts.project);
  const cargoToml = readText(path.join(srcTauri, "Cargo.toml"));
  const backend = pluginBackend(cargoToml);
  if (!backend) throw new Error("tauri-plugin-python is not a dependency in src-tauri/Cargo.toml");
  if (backend === "rustpython") {
    log("This app uses the RustPython backend, which needs no Python install. Nothing to do.");
    return;
  }
  const triple = hostTriple();
  const os = osOf(triple);
  const manifestFile = path.join(srcTauri, MANIFEST);
  /** @type {Manifest | undefined} */
  let manifest = readJson(manifestFile);
  const pythonDir = path.join(srcTauri, PYTHON_DIR);
  const wanted = opts.python ?? pinnedPythonVersion(srcTauri) ?? manifest?.python ?? DEFAULT_PYTHON;

  if (fs.existsSync(pythonDir) && !manifest && !opts.force) {
    throw new Error(`${PYTHON_DIR} exists but was not created by this tool; use --force to replace it`);
  }
  const release = isVersionAlias(wanted) ? await fetchRelease(opts.release) : undefined;
  const resolved = release ? selectAsset(Object.keys(release.assets), wanted, triple).version : wanted;
  const installed = manifest && fs.existsSync(path.join(srcTauri, layout(os, manifest.python).stdlib));
  if (opts.force || !manifest || !installed || !matches(manifest, resolved, triple, opts.release)) {
    const rel = release ?? (await fetchRelease(opts.release));
    const asset = selectAsset(Object.keys(rel.assets), resolved, triple);
    const archive = await download(rel, asset.name, log);
    fs.mkdirSync(path.join(srcTauri, EMBED_DIR), { recursive: true });
    extract(archive, pythonDir);
    prune(pythonDir, layout(os, asset.version).prune);
    manifest = { python: asset.version, release: asset.release, triple, asset: asset.name };
    writeJson(manifestFile, manifest);
    log(`Installed Python ${asset.version} (${asset.release}) to ${PYTHON_DIR}`);
  } else {
    log(`Python ${manifest.python} already installed in ${PYTHON_DIR}`);
  }

  if (opts.python || !pinnedPythonVersion(srcTauri)) {
    const pin = isVersionAlias(wanted) ? manifest.python.split(".").slice(0, 2).join(".") : wanted;
    const pinFile = writePythonPin(srcTauri, pin);
    if (pinFile) log(`Pinned Python ${pin} in ${path.relative(srcTauri, pinFile)}; commit it so CI and the team use it too`);
  }

  const l = layout(os, manifest.python);
  const requirements = opts.requirements ?? path.join(srcTauri, "src-python", "requirements.txt");
  if (fs.existsSync(requirements)) {
    log(`Installing ${path.relative(srcTauri, requirements)} ...`);
    run(path.join(srcTauri, l.python), ["-I", "-m", "pip", "install", "--disable-pip-version-check", "-r", requirements]);
  }

  fs.writeFileSync(path.join(srcTauri, PYO3_CONFIG), pyo3Config(srcTauri, os, manifest.python));
  writeCargoConfig(srcTauri, productName(readJson(path.join(srcTauri, "tauri.conf.json")), cargoToml));
  for (const [src, dest] of Object.entries(l.copies)) {
    if (fs.existsSync(path.join(srcTauri, src))) fs.copyFileSync(path.join(srcTauri, src), path.join(srcTauri, dest));
  }
  writeTauriConfigs(srcTauri, os, l.resources);
  const gitignore = path.join(srcTauri, ".gitignore");
  fs.writeFileSync(gitignore, withGitignoreEntry(readText(gitignore), `/${EMBED_DIR}/`));

  const mismatch = venvMismatch(srcTauri, manifest.python);
  if (mismatch) log(`Warning: ${mismatch}`);
  if (!bundlesSrcPython(mergedResources(srcTauri, os))) {
    log(`Warning: src-python/ is not in bundle.resources of tauri.conf.json; release builds won't find main.py`);
  }

  log("");
  log("Done. The app now uses the bundled Python in dev and release builds.");
  log(`Run this command again after cloning the project or changing requirements; ${EMBED_DIR}/ is not committed.`);
  if (os === "macos") log("Before notarizing, sign the bundled Python: npx tauri-plugin-python-cli sign");
}

/**
 * Whether the installed Python satisfies the requested version prefix.
 * @param {Manifest} manifest
 * @param {string} wanted
 * @param {string} triple
 * @param {string | undefined} release
 */
function matches(manifest, wanted, triple, release) {
  const want = wanted.split(".");
  const have = manifest.python.split(".");
  return (
    manifest.triple === triple &&
    want.every((part, i) => part === have[i]) &&
    (!release || release === manifest.release)
  );
}

/**
 * @param {string} pythonDir
 * @param {string[]} patterns  relative paths, `*` allowed in the last segment
 */
function prune(pythonDir, patterns) {
  for (const pattern of patterns) {
    const dir = path.join(pythonDir, path.dirname(pattern));
    const base = path.basename(pattern);
    if (!fs.existsSync(dir)) continue;
    const re = new RegExp(`^${base.replace(/[.]/g, "\\.").replace(/\*/g, ".*")}$`);
    for (const name of fs.readdirSync(dir).filter((n) => re.test(n))) {
      fs.rmSync(path.join(dir, name), { recursive: true, force: true });
    }
  }
}

/**
 * @param {string} cmd
 * @param {string[]} args
 */
function run(cmd, args) {
  const res = spawnSync(cmd, args, { stdio: "inherit" });
  if (res.status !== 0) throw new Error(`${path.basename(cmd)} ${args.join(" ")} failed`);
}

/**
 * @param {string} srcTauri
 * @param {string | undefined} product
 */
function writeCargoConfig(srcTauri, product) {
  const file = path.join(srcTauri, ".cargo", "config.toml");
  const existing = readText(file);
  const block = cargoConfigBlock(product);
  const conflicts = existing ? cargoConfigConflicts(existing) : [];
  if (conflicts.length) {
    throw new Error(
      `.cargo/config.toml already sets ${conflicts.join(" and ")}. Merge this into it manually:\n\n${block}`,
    );
  }
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, upsertCargoConfig(existing, block));
}

/**
 * Resources differ per OS, so they go into `tauri.<os>.conf.json`. JSON Merge
 * Patch only combines objects, so the base resources are converted as well.
 * @param {string} srcTauri
 * @param {import("./platform.js").Os} os
 * @param {Record<string, string>} resources
 */
function writeTauriConfigs(srcTauri, os, resources) {
  const baseFile = path.join(srcTauri, "tauri.conf.json");
  /** @type {import("./configs.js").TauriConf} */
  const base = readJson(baseFile);
  if (base.bundle && Array.isArray(base.bundle.resources)) {
    const manual = unconvertibleResources(base.bundle.resources);
    if (manual.length) {
      throw new Error(
        `bundle.resources in tauri.conf.json must be an object so tauri can merge the Python resources, ` +
          `but these entries can't be converted automatically: ${manual.join(", ")}. ` +
          `Convert them to the "source": "target" form and run setup again`,
      );
    }
    base.bundle.resources = resourcesAsObject(base.bundle.resources);
    writeJson(baseFile, base);
  }
  const existing = Object.fromEntries(
    Object.entries(resources).filter(([src]) => fs.existsSync(path.join(srcTauri, src))),
  );
  const platformFile = path.join(srcTauri, `tauri.${os}.conf.json`);
  const platformRes = readJson(platformFile)?.bundle?.resources;
  const manualPlatform = Array.isArray(platformRes) ? unconvertibleResources(platformRes) : [];
  if (manualPlatform.length) {
    throw new Error(
      `bundle.resources in tauri.${os}.conf.json can't be converted to the object form automatically: ` +
        `${manualPlatform.join(", ")}. Convert them to the "source": "target" form and run setup again`,
    );
  }
  /** @param {string} src */
  const isOurs = (src) => src.startsWith(`${EMBED_DIR}/`);
  writeJson(platformFile, withPythonResources(readJson(platformFile), existing, isOurs));
}
