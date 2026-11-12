// @ts-check
// Generation and merging of the project files the bundled Python needs.

import { PYO3_CONFIG, rpaths } from "./platform.js";

/** @typedef {Record<string, string>} Resources */
/** @typedef {{ bundle?: { resources?: Resources | string[] } } & Record<string, unknown>} TauriConf */

export const BLOCK_START = "# >>> tauri-plugin-python (managed by tauri-plugin-python-cli)";
export const BLOCK_END = "# <<< tauri-plugin-python";

/**
 * @param {string | undefined} productName
 * @returns {string}
 */
export function cargoConfigBlock(productName) {
  /** @param {import("./platform.js").Os} os */
  const flags = (os) =>
    rpaths(os, productName)
      .flatMap((p) => ["-C", `link-arg=-Wl,-rpath,${p}`])
      .map((f) => JSON.stringify(f))
      .join(", ");
  return [
    BLOCK_START,
    "[env]",
    `PYO3_CONFIG_FILE = { value = "${PYO3_CONFIG}", relative = true }`,
    "",
    `[target.'cfg(target_os = "macos")']`,
    `rustflags = [${flags("macos")}]`,
    "",
    `[target.'cfg(target_os = "linux")']`,
    `rustflags = [${flags("linux")}]`,
    BLOCK_END,
    "",
  ].join("\n");
}

/**
 * @param {string} text
 * @returns {{ user: string, block: string | null }}
 */
function splitManagedBlock(text) {
  const start = text.indexOf(BLOCK_START);
  const end = text.indexOf(BLOCK_END);
  if (start === -1 || end === -1 || end < start) return { user: text, block: null };
  const after = text.slice(end + BLOCK_END.length).replace(/^\r?\n/, "");
  return { user: text.slice(0, start) + after, block: text.slice(start, end + BLOCK_END.length) };
}

/** @param {string} text */
export function hasManagedBlock(text) {
  return splitManagedBlock(text).block !== null;
}

/**
 * Settings outside the managed block that would collide with it.
 * @param {string} text
 * @returns {string[]}
 */
export function cargoConfigConflicts(text) {
  const { user } = splitManagedBlock(text);
  const conflicts = [];
  if (/^\s*\[env\]/m.test(user) || /^\s*env\./m.test(user)) conflicts.push("[env]");
  if (/rustflags/.test(user)) conflicts.push("rustflags");
  return conflicts;
}

/**
 * Replaces the managed block in an existing `.cargo/config.toml` or appends it.
 * @param {string | undefined} existing
 * @param {string} block
 * @returns {string}
 */
export function upsertCargoConfig(existing, block) {
  const { user } = splitManagedBlock(existing ?? "");
  const head = user.trimEnd();
  return head ? `${head}\n\n${block}` : block;
}

/**
 * Tauri merges `tauri.<os>.conf.json` into `tauri.conf.json` via JSON Merge
 * Patch, so resources must be an object in both to be combined.
 * @param {Resources | string[] | undefined | null} resources
 * @returns {Resources}
 */
export function resourcesAsObject(resources) {
  if (resources === undefined || resources === null) return {};
  if (Array.isArray(resources)) return Object.fromEntries(resources.map((r) => [r, r]));
  return { ...resources };
}

/**
 * List entries whose meaning changes in the object form: tauri maps `../` to
 * `_up_/` and absolute paths to their file name only in the list form, and
 * globs keep their directory structure only there.
 * @param {string[]} resources
 * @returns {string[]}
 */
export function unconvertibleResources(resources) {
  return resources.filter((r) => /[*?[{]/.test(r) || /(^|[\\/])\.\.([\\/]|$)/.test(r) || /^([a-zA-Z]:)?[\\/]/.test(r));
}

/**
 * Returns `platformConf` with `resources` added and older entries matching
 * `isOurs` removed.
 * @param {TauriConf | undefined} platformConf
 * @param {Resources} resources
 * @param {(src: string) => boolean} isOurs
 * @returns {TauriConf}
 */
export function withPythonResources(platformConf, resources, isOurs) {
  /** @type {TauriConf} */
  const conf = structuredClone(platformConf ?? {});
  conf.bundle ??= {};
  const current = resourcesAsObject(conf.bundle.resources);
  const kept = Object.fromEntries(Object.entries(current).filter(([src]) => !isOurs(src)));
  conf.bundle.resources = { ...kept, ...resources };
  return conf;
}

/**
 * @param {string | undefined} text
 * @param {string} entry
 * @returns {string}
 */
export function withGitignoreEntry(text, entry) {
  const content = text ?? "";
  if (content.split(/\r?\n/).some((l) => l.trim() === entry)) return content;
  const head = content.trimEnd();
  return head ? `${head}\n${entry}\n` : `${entry}\n`;
}
