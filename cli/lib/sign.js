// @ts-check
// `sign`: codesigns the native files of the bundled Python on macOS.
// Notarization rejects unsigned Mach-O files anywhere in the app, and tauri only
// signs the app binary and frameworks, not files copied as resources.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { PYTHON_DIR } from "./platform.js";
import { findSrcTauri } from "./project.js";

const MACHO_MAGIC = new Set(["feedfacf", "cffaedfe", "cafebabe", "bebafeca"]);

/** @param {string} file */
export function isMachO(file) {
  const fd = fs.openSync(file, "r");
  try {
    const buf = Buffer.alloc(4);
    return fs.readSync(fd, buf, 0, 4, 0) === 4 && MACHO_MAGIC.has(buf.toString("hex"));
  } finally {
    fs.closeSync(fd);
  }
}

/**
 * @param {string} dir
 * @returns {Generator<string>}
 */
function* files(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) yield* files(p);
    else if (entry.isFile()) yield p;
  }
}

/**
 * @typedef {object} SignOptions
 * @property {string} [project]
 * @property {string} [identity]      codesign identity, "-" for ad-hoc
 * @property {string} [entitlements]
 */

/**
 * @param {SignOptions} opts
 * @param {(msg: string) => void} [log]
 */
export function sign(opts, log = console.log) {
  if (process.platform !== "darwin") throw new Error("'sign' is only needed (and possible) on macOS");
  const identity = opts.identity ?? process.env.APPLE_SIGNING_IDENTITY;
  if (!identity) throw new Error("Pass --identity or set APPLE_SIGNING_IDENTITY (as for tauri build)");
  const pythonDir = path.join(findSrcTauri(opts.project), PYTHON_DIR);
  if (!fs.existsSync(pythonDir)) throw new Error(`${PYTHON_DIR} not found. Run 'setup' first`);

  const targets = [...files(pythonDir)].filter(isMachO);
  const args = ["--force", "--sign", identity];
  if (identity !== "-") args.push("--timestamp", "--options", "runtime");
  if (opts.entitlements) args.push("--entitlements", opts.entitlements);
  for (const file of targets) {
    const res = spawnSync("codesign", [...args, file], { stdio: "inherit" });
    if (res.status !== 0) throw new Error(`codesign failed for ${file}`);
  }
  log(`Signed ${targets.length} files in ${PYTHON_DIR}`);
}
