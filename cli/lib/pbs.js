// @ts-check
// Download of python-build-standalone (https://github.com/astral-sh/python-build-standalone).

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { parseVersion } from "./platform.js";

/**
 * @typedef {object} Release
 * @property {string} tag
 * @property {Record<string, string>} assets  asset name -> download url
 */

/**
 * @typedef {object} Asset
 * @property {string} name
 * @property {string} version  full Python version, e.g. 3.13.18
 * @property {string} release  python-build-standalone release tag
 * @property {number} patch
 * @property {boolean} stripped
 */

const REPO = "astral-sh/python-build-standalone";

const ASSET_RE = /^cpython-(\d+)\.(\d+)\.(\d+)\+(\d+)-(.+?)-(install_only_stripped|install_only)\.tar\.gz$/;

/** `stable` moves to a new minor version once it reached this patch level (x.y.2, ~4 months after x.y.0). */
export const STABLE_MIN_PATCH = 2;

/** @param {string} version */
export const isVersionAlias = (version) => version === "stable" || version === "latest";

/**
 * Picks the newest patch release of `version` (e.g. "3.13") for `triple`,
 * preferring the stripped build. An exact "3.13.5" pins the patch level.
 * `latest` is the newest minor version in the release, `stable` the newest
 * one that reached {@link STABLE_MIN_PATCH}.
 * @param {string[]} assetNames
 * @param {string} version
 * @param {string} triple
 * @returns {Asset}
 */
export function selectAsset(assetNames, version, triple) {
  /** @type {(Asset & { minor: string })[]} */
  const builds = [];
  for (const name of assetNames) {
    const m = ASSET_RE.exec(name);
    if (!m || m[5] !== triple) continue;
    builds.push({
      name,
      version: `${m[1]}.${m[2]}.${m[3]}`,
      minor: `${m[1]}.${m[2]}`,
      release: m[4],
      patch: +m[3],
      stripped: m[6] === "install_only_stripped",
    });
  }
  const resolved = isVersionAlias(version) ? resolveAlias(builds, version) : version;
  const want = resolved === undefined ? undefined : parseVersion(resolved);
  const candidates = builds.filter(
    (b) =>
      want &&
      b.minor === `${want.major}.${want.minor}` &&
      (want.patch === undefined || b.patch === want.patch),
  );
  candidates.sort((a, b) => b.patch - a.patch || Number(b.stripped) - Number(a.stripped));
  if (!candidates.length) {
    throw new Error(`No python-build-standalone build of Python ${version} for ${triple}`);
  }
  const { minor, ...asset } = candidates[0];
  return asset;
}

/**
 * @param {{ minor: string, patch: number }[]} builds
 * @param {"stable" | "latest"} alias
 * @returns {string | undefined}  e.g. "3.14"
 */
function resolveAlias(builds, alias) {
  /** @type {Map<string, number>} */
  const newestPatch = new Map();
  for (const b of builds) newestPatch.set(b.minor, Math.max(newestPatch.get(b.minor) ?? -1, b.patch));
  /** @param {string} v */
  const key = (v) => v.split(".").map(Number);
  const minors = [...newestPatch].sort(([a], [b]) => key(b)[0] - key(a)[0] || key(b)[1] - key(a)[1]);
  const pick = alias === "latest" ? minors[0] : (minors.find(([, p]) => p >= STABLE_MIN_PATCH) ?? minors[0]);
  return pick?.[0];
}

/**
 * @param {string} text  contents of a `SHA256SUMS` file
 * @returns {Record<string, string>}  file name -> lowercase hex digest
 */
export function parseSha256Sums(text) {
  return Object.fromEntries(
    text
      .split(/\r?\n/)
      .map((l) => l.trim().split(/\s+/))
      .filter((p) => p.length === 2)
      .map(([hash, name]) => [name.replace(/^\*/, ""), hash.toLowerCase()]),
  );
}

/**
 * @param {string} url
 * @param {string} accept
 */
async function get(url, accept) {
  /** @type {Record<string, string>} */
  const headers = { "User-Agent": "tauri-plugin-python-cli", Accept: accept };
  if (process.env.GITHUB_TOKEN && url.startsWith("https://api.github.com/")) {
    headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  }
  const res = await fetch(url, { headers });
  if (!res.ok) throw new Error(`GET ${url} failed: ${res.status} ${res.statusText}`);
  return res;
}

/**
 * @param {string} [tag]  release tag, latest if omitted
 * @returns {Promise<Release>}
 */
export async function fetchRelease(tag) {
  const which = tag ? `tags/${tag}` : "latest";
  const res = await get(`https://api.github.com/repos/${REPO}/releases/${which}`, "application/vnd.github+json");
  /** @type {{ tag_name: string, assets: { name: string, browser_download_url: string }[] }} */
  const release = await res.json();
  return {
    tag: release.tag_name,
    assets: Object.fromEntries(release.assets.map((a) => [a.name, a.browser_download_url])),
  };
}

export function cacheDir() {
  const base =
    process.env.TAURI_PLUGIN_PYTHON_CACHE ??
    (process.platform === "win32" ? process.env.LOCALAPPDATA : undefined) ??
    path.join(os.homedir(), ".cache");
  return path.join(base, "tauri-plugin-python-cli");
}

/**
 * Downloads `asset` into the cache (once) and verifies its SHA256.
 * @param {Release} release
 * @param {string} asset
 * @param {(msg: string) => void} log
 * @returns {Promise<string>}  path of the archive
 */
export async function download(release, asset, log) {
  const dir = cacheDir();
  fs.mkdirSync(dir, { recursive: true });
  const file = path.join(dir, asset);
  const sumsUrl = release.assets.SHA256SUMS;
  const expected = sumsUrl ? parseSha256Sums(await (await get(sumsUrl, "*/*")).text())[asset] : undefined;
  if (!expected) throw new Error(`No SHA256 checksum published for ${asset}`);
  if (fs.existsSync(file) && sha256(file) === expected) {
    log(`Using cached ${asset}`);
    return file;
  }
  log(`Downloading ${asset} ...`);
  const res = await get(release.assets[asset], "application/octet-stream");
  const tmp = `${file}.${process.pid}.tmp`;
  fs.writeFileSync(tmp, Buffer.from(await res.arrayBuffer()));
  const actual = sha256(tmp);
  if (actual !== expected) {
    fs.rmSync(tmp, { force: true });
    throw new Error(`Checksum mismatch for ${asset}: expected ${expected}, got ${actual}`);
  }
  fs.renameSync(tmp, file);
  return file;
}

/** @param {string} file */
function sha256(file) {
  return createHash("sha256").update(fs.readFileSync(file)).digest("hex");
}

/**
 * GNU tar from Git Bash or MSYS reads `C:\...` as a remote host, so prefer the
 * bsdtar that ships with Windows.
 */
function tarCommand() {
  if (process.platform !== "win32") return "tar";
  const systemTar = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
  return fs.existsSync(systemTar) ? systemTar : "tar";
}

/**
 * Extracts the archive's top-level `python/` dir to `dest`.
 * @param {string} archive
 * @param {string} dest
 */
export function extract(archive, dest) {
  const tmp = fs.mkdtempSync(path.join(path.dirname(dest), ".extract-"));
  try {
    const res = spawnSync(tarCommand(), ["-xzf", archive, "-C", tmp], { stdio: "inherit" });
    if (res.status !== 0) throw new Error(`Extracting ${archive} failed (is 'tar' installed?)`);
    fs.rmSync(dest, { recursive: true, force: true });
    fs.renameSync(path.join(tmp, "python"), dest);
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
}
