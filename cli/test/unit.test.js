// @ts-check
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";
import {
  BLOCK_END,
  BLOCK_START,
  cargoConfigBlock,
  cargoConfigConflicts,
  hasManagedBlock,
  resourcesAsObject,
  unconvertibleResources,
  upsertCargoConfig,
  withGitignoreEntry,
  withPythonResources,
} from "../lib/configs.js";
import { doctor } from "../lib/doctor.js";
import { parseSha256Sums, selectAsset } from "../lib/pbs.js";
import { hostTriple, layout, osOf, parseVersion, pyo3Config, rpaths } from "../lib/platform.js";
import {
  duplicateLockVersions,
  hasCompiledEntry,
  packageName,
  pinnedPythonVersion,
  pluginBackend,
  productName,
  venvMismatch,
  writePythonPin,
} from "../lib/project.js";

test("host triples", () => {
  assert.equal(hostTriple("darwin", "arm64"), "aarch64-apple-darwin");
  assert.equal(hostTriple("linux", "x64"), "x86_64-unknown-linux-gnu");
  assert.equal(hostTriple("win32", "x64"), "x86_64-pc-windows-msvc");
  assert.throws(() => hostTriple("aix", "ppc64"));
  assert.equal(osOf("x86_64-pc-windows-msvc"), "windows");
  assert.equal(osOf("aarch64-apple-darwin"), "macos");
  assert.equal(osOf("x86_64-unknown-linux-gnu"), "linux");
});

test("version parsing", () => {
  assert.deepEqual(parseVersion("3.13"), { major: 3, minor: 13, patch: undefined });
  assert.deepEqual(parseVersion("3.12.7"), { major: 3, minor: 12, patch: 7 });
  assert.throws(() => parseVersion("3"));
  assert.throws(() => parseVersion("latest"));
});

test("unix layout bundles only stdlib and libpython", () => {
  const mac = layout("macos", "3.13.18");
  assert.equal(mac.python, "pyembed/python/bin/python3.13");
  assert.equal(mac.libName, "python3.13");
  assert.deepEqual(Object.keys(mac.resources), [
    "pyembed/python/lib/python3.13/",
    "pyembed/python/lib/libpython3.13.dylib",
  ]);
  const linux = layout("linux", "3.12");
  assert.ok("pyembed/python/lib/libpython3.12.so.1.0" in linux.resources);
});

test("windows layout puts the DLLs next to the exe", () => {
  const win = layout("windows", "3.13");
  assert.equal(win.python, "pyembed/python/python.exe");
  assert.equal(win.libName, "python313");
  assert.equal(win.libDir, "pyembed/python/libs");
  assert.equal(win.resources["pyembed/python/python313.dll"], "python313.dll");
  assert.equal(win.resources["pyembed/python/python3.dll"], "python3.dll");
  assert.equal(win.resources["pyembed/python/"], "pyembed/python/");
});

test("rpaths cover dev builds and bundles", () => {
  assert.deepEqual(rpaths("macos", "App"), [
    "@executable_path/../Resources/pyembed/python/lib",
    "@executable_path/pyembed/python/lib",
  ]);
  assert.deepEqual(rpaths("linux", "my-app"), [
    "$ORIGIN/../lib/my-app/pyembed/python/lib",
    "$ORIGIN/pyembed/python/lib",
  ]);
  assert.deepEqual(rpaths("windows", "x"), []);
});

test("pyo3 config uses absolute paths", () => {
  const root = path.resolve("/project/src-tauri");
  const cfg = pyo3Config(root, "macos", "3.13.18");
  assert.match(cfg, /^version=3\.13$/m);
  assert.match(cfg, /^lib_name=python3\.13$/m);
  assert.ok(cfg.includes(`lib_dir=${path.join(root, "pyembed/python/lib")}`));
  assert.ok(cfg.endsWith("\n"));
});

test("cargo config block is TOML with both rpaths", () => {
  const block = cargoConfigBlock("my-app");
  assert.ok(block.startsWith(BLOCK_START));
  assert.ok(block.trimEnd().endsWith(BLOCK_END));
  assert.match(block, /PYO3_CONFIG_FILE = \{ value = "pyembed\/pyo3-config.txt", relative = true \}/);
  assert.ok(block.includes('"link-arg=-Wl,-rpath,$ORIGIN/../lib/my-app/pyembed/python/lib"'));
  assert.ok(block.includes('"link-arg=-Wl,-rpath,@executable_path/pyembed/python/lib"'));
});

test("managed cargo block is appended once and replaced on update", () => {
  const user = '[build]\ntarget-dir = "out"\n';
  const first = upsertCargoConfig(user, cargoConfigBlock("a"));
  assert.ok(first.startsWith(user.trimEnd()));
  assert.ok(hasManagedBlock(first));
  const second = upsertCargoConfig(first, cargoConfigBlock("b"));
  assert.equal(second.split(BLOCK_START).length, 2);
  assert.ok(second.includes("/lib/b/"));
  assert.ok(!second.includes("/lib/a/"));
  assert.ok(second.startsWith(user.trimEnd()));
  assert.equal(upsertCargoConfig(undefined, "X"), "X");
});

test("conflicting cargo settings outside the block are reported", () => {
  assert.deepEqual(cargoConfigConflicts(upsertCargoConfig("", cargoConfigBlock("a"))), []);
  assert.deepEqual(cargoConfigConflicts('[env]\nFOO = "1"\n'), ["[env]"]);
  assert.deepEqual(cargoConfigConflicts('[build]\nrustflags = ["-g"]\n'), ["rustflags"]);
  assert.deepEqual(cargoConfigConflicts('env.FOO = "1"\n'), ["[env]"]);
});

test("resources are converted to the object form", () => {
  assert.deepEqual(resourcesAsObject(["src-python/"]), { "src-python/": "src-python/" });
  assert.deepEqual(resourcesAsObject({ a: "b" }), { a: "b" });
  assert.deepEqual(resourcesAsObject(undefined), {});
});

test("python resources replace older python entries in the platform config", () => {
  /** @param {string} src */
  const isOurs = (src) => src.startsWith("pyembed/python/");
  const old = {
    identifier: "x",
    bundle: { resources: { "other/": "other/", "pyembed/python/lib/python3.12/": "pyembed/python/lib/python3.12/" } },
  };
  const conf = withPythonResources(old, { "pyembed/python/lib/python3.13/": "pyembed/python/lib/python3.13/" }, isOurs);
  assert.deepEqual(conf.bundle?.resources, {
    "other/": "other/",
    "pyembed/python/lib/python3.13/": "pyembed/python/lib/python3.13/",
  });
  assert.equal(conf.identifier, "x");
  assert.ok("pyembed/python/lib/python3.12/" in old.bundle.resources, "input is not mutated");
  assert.deepEqual(withPythonResources(undefined, { a: "a" }, isOurs), { bundle: { resources: { a: "a" } } });
});

test("gitignore entry is added once", () => {
  assert.equal(withGitignoreEntry(undefined, "/pyembed/"), "/pyembed/\n");
  assert.equal(withGitignoreEntry("/target/\n", "/pyembed/"), "/target/\n/pyembed/\n");
  assert.equal(withGitignoreEntry("/target/\n/pyembed/\n", "/pyembed/"), "/target/\n/pyembed/\n");
});

test("newest stripped build of the requested minor version is selected", () => {
  const t = "aarch64-apple-darwin";
  const names = [
    `cpython-3.13.17+20261110-${t}-install_only_stripped.tar.gz`,
    `cpython-3.13.18+20261110-${t}-install_only.tar.gz`,
    `cpython-3.13.18+20261110-${t}-install_only_stripped.tar.gz`,
    `cpython-3.13.18+20261110-${t}-freethreaded-install_only_stripped.tar.gz`,
    `cpython-3.13.18+20261110-${t}-pgo+lto-full.tar.zst`,
    `cpython-3.14.2+20261110-${t}-install_only_stripped.tar.gz`,
    `cpython-3.13.18+20261110-x86_64_v3-unknown-linux-gnu-install_only_stripped.tar.gz`,
    "SHA256SUMS",
  ];
  const a = selectAsset(names, "3.13", t);
  assert.equal(a.name, `cpython-3.13.18+20261110-${t}-install_only_stripped.tar.gz`);
  assert.equal(a.version, "3.13.18");
  assert.equal(a.release, "20261110");
  assert.equal(selectAsset(names, "3.13.17", t).version, "3.13.17");
  assert.equal(selectAsset(names, "3.14", t).version, "3.14.2");
  assert.throws(() => selectAsset(names, "3.12", t), /No python-build-standalone build/);
  assert.throws(() => selectAsset(names, "3.13", "x86_64-unknown-linux-gnu"));
});

test("stable and latest resolve to a minor version", () => {
  const t = "aarch64-apple-darwin";
  /** @param {string} v */
  const asset = (v) => `cpython-${v}+20261110-${t}-install_only_stripped.tar.gz`;
  const names = ["3.12.12", "3.13.9", "3.14.1", "3.14.2", "3.15.0"].map(asset);
  assert.equal(selectAsset(names, "latest", t).version, "3.15.0");
  assert.equal(selectAsset(names, "stable", t).version, "3.14.2");
  assert.equal(selectAsset(names.slice(0, 3), "stable", t).version, "3.13.9");
  assert.equal(selectAsset([asset("3.15.0")], "stable", t).version, "3.15.0");
  assert.throws(() => selectAsset([], "stable", t), /No python-build-standalone build/);
});

test("sha256 sums are parsed", () => {
  const sums = parseSha256Sums("ABC123  file-a.tar.gz\ndef456 *file-b.tar.gz\n\n");
  assert.deepEqual(sums, { "file-a.tar.gz": "abc123", "file-b.tar.gz": "def456" });
});

test("plugin backend detection", () => {
  assert.equal(pluginBackend('[dependencies]\ntauri-plugin-python = "0.4"\n'), "pyo3");
  assert.equal(
    pluginBackend('tauri-plugin-python = { version = "0.4", default-features = false, features = ["venv", "rustpython"] }'),
    "rustpython",
  );
  assert.equal(
    pluginBackend('[dependencies.tauri-plugin-python]\nversion = "0.4"\nfeatures = ["rustpython"]\n'),
    "rustpython",
  );
  assert.equal(pluginBackend('[dependencies]\ntauri = "2"\n'), undefined);
});

test("package and product names", () => {
  const toml = '[package]\nname = "my_app"\nversion = "0.1.0"\n\n[dependencies]\nname = "x"\n';
  assert.equal(packageName(toml), "my_app");
  assert.equal(productName({ productName: "My App" }, toml), "My App");
  assert.equal(productName({}, toml), "my_app");
});

test("duplicate lockfile versions", () => {
  /** @param {string} name @param {string} v */
  const lock = (name, v) => `[[package]]\nname = "${name}"\nversion = "${v}"\nsource = "x"\n\n`;
  const text = lock("malachite-bigint", "0.9.2") + lock("foo", "1.0.0") + lock("malachite-bigint", "0.14.0");
  assert.deepEqual(duplicateLockVersions(text, "malachite-bigint"), ["0.9.2", "0.14.0"]);
  assert.deepEqual(duplicateLockVersions(text, "foo"), []);
});

test("compiled entry detection", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tpp-"));
  try {
    assert.equal(hasCompiledEntry(dir), false);
    fs.writeFileSync(path.join(dir, "helper.so"), "");
    assert.equal(hasCompiledEntry(dir), false);
    fs.writeFileSync(path.join(dir, "main.cpython-313-darwin.so"), "");
    assert.equal(hasCompiledEntry(dir), true);
    assert.equal(hasCompiledEntry(path.join(dir, "missing")), false);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test("resources that change meaning in the object form are not converted", () => {
  assert.deepEqual(
    unconvertibleResources(["src-python/", "icons/a.png", "../x.json", "/abs/a", "C:\\x", "a/**/*.md", "a/../b"]),
    ["../x.json", "/abs/a", "C:\\x", "a/**/*.md", "a/../b"],
  );
});

test("doctor: an object in tauri.<os>.conf.json replaces list resources of tauri.conf.json", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tpp-doctor-"));
  try {
    fs.mkdirSync(path.join(dir, "src-python"));
    fs.writeFileSync(path.join(dir, "src-python", "main.py"), "");
    fs.writeFileSync(path.join(dir, "Cargo.toml"), '[package]\nname = "app"\n\n[dependencies]\ntauri-plugin-python = "0.4"\n');
    fs.writeFileSync(path.join(dir, "tauri.conf.json"), JSON.stringify({ bundle: { resources: ["src-python/"] } }));
    const osName = osOf(hostTriple());
    fs.writeFileSync(
      path.join(dir, `tauri.${osName}.conf.json`),
      JSON.stringify({ bundle: { resources: { "pyembed/python/lib/": "pyembed/python/lib/" } } }),
    );
    const findings = doctor({ project: dir });
    assert.ok(findings.some((f) => f.level === "error" && f.msg.startsWith("src-python/ is missing")));
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test("venv created with another Python version is reported", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tpp-venv-"));
  try {
    assert.equal(venvMismatch(dir, "3.13.18"), undefined);
    const venv = path.join(dir, "src-python", ".venv");
    fs.mkdirSync(venv, { recursive: true });
    fs.writeFileSync(path.join(venv, "pyvenv.cfg"), "home = /usr/bin\nversion = 3.13.2\n");
    assert.equal(venvMismatch(dir, "3.13.18"), undefined);
    fs.writeFileSync(path.join(venv, "pyvenv.cfg"), "home = /usr/bin\nversion_info = 3.14.0\n");
    assert.match(venvMismatch(dir, "3.13.18") ?? "", /Python 3\.14, the bundled Python is 3\.13/);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test("python version is read from src-python/.python-version", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tpp-pin-"));
  try {
    assert.equal(pinnedPythonVersion(dir), undefined);
    assert.equal(writePythonPin(dir, "3.14"), undefined);
    fs.mkdirSync(path.join(dir, "src-python"));
    fs.writeFileSync(path.join(dir, "src-python", ".python-version"), "# comment\n\n 3.14 \n");
    assert.equal(pinnedPythonVersion(dir), "3.14");
    assert.equal(writePythonPin(dir, "3.14"), undefined);
    assert.ok(writePythonPin(dir, "3.15"));
    assert.equal(pinnedPythonVersion(dir), "3.15");
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
