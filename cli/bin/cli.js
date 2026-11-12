#!/usr/bin/env node
// @ts-check
import { parseArgs } from "node:util";
import { doctor } from "../lib/doctor.js";
import { DEFAULT_PYTHON, setup } from "../lib/setup.js";
import { sign } from "../lib/sign.js";

const HELP = `Usage: tauri-plugin-python-cli <command> [options]

Commands:
  setup    Bundle a standalone Python (PyO3 backend) into src-tauri/pyembed,
           install src-python/requirements.txt into it and configure linking
           and tauri resources.
  doctor   Check the project for common deployment problems.
  sign     Codesign the bundled Python on macOS (needed for notarization).

Options:
  --project <dir>        Project root or src-tauri dir (default: cwd)
  --python <version>     setup: 3.13, 3.13.5, stable or latest; written to src-python/.python-version
                         (default: that file, else ${DEFAULT_PYTHON})
  --release <tag>        setup: python-build-standalone release (default: latest)
  --requirements <file>  setup: requirements file (default: src-python/requirements.txt)
  --force                setup: download and replace the bundled Python
  --fix                  doctor: apply automatic fixes
  --identity <name>      sign: signing identity (default: $APPLE_SIGNING_IDENTITY)
  --entitlements <file>  sign: entitlements plist
  -h, --help             Show this help
`;

/** @type {Record<import("../lib/doctor.js").Finding["level"], string>} */
const ICONS = { ok: "✔", warn: "!", error: "✖" };

async function main() {
  const { values, positionals } = parseArgs({
    allowPositionals: true,
    options: {
      project: { type: "string" },
      python: { type: "string" },
      release: { type: "string" },
      requirements: { type: "string" },
      force: { type: "boolean" },
      fix: { type: "boolean" },
      identity: { type: "string" },
      entitlements: { type: "string" },
      help: { type: "boolean", short: "h" },
    },
  });
  const [command] = positionals;
  if (values.help || !command) {
    console.log(HELP);
    return 0;
  }
  if (command === "setup") {
    await setup(values);
    return 0;
  }
  if (command === "sign") {
    sign(values);
    return 0;
  }
  if (command === "doctor") {
    const findings = doctor(values);
    for (const f of findings) console.log(`${ICONS[f.level]} ${f.msg}`);
    return findings.some((f) => f.level === "error") ? 1 : 0;
  }
  console.error(`Unknown command '${command}'\n\n${HELP}`);
  return 2;
}

main().then(
  (code) => {
    process.exitCode = code;
  },
  (err) => {
    console.error(`Error: ${err.message}`);
    process.exitCode = 1;
  },
);
