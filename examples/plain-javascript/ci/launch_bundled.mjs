// Starts an installed app with a minimal environment, waits for the report of
// ci/probe_main.py and checks that it ran the bundled Python.
// Usage: node launch_bundled.mjs <executable> <report-file>
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const [exe, report] = process.argv.slice(2);
const passthrough = ["DISPLAY", "XAUTHORITY", "SystemRoot", "SystemDrive", "TEMP", "TMP", "LOCALAPPDATA", "APPDATA", "USERPROFILE", "HOME"];
const env = Object.fromEntries(passthrough.filter((k) => process.env[k]).map((k) => [k, process.env[k]]));
env.E2E_REPORT = report;

fs.rmSync(report, { force: true });
const child = spawn(exe, [], { env, cwd: path.dirname(report), stdio: "inherit" });
let exited = false;
child.on("exit", (code) => {
  exited = true;
  console.log(`app exited with ${code} (0x${(code >>> 0).toString(16)})`);
});

function fail(msg) {
  console.error(`FAIL: ${msg}`);
  console.error(`exe: ${exe}\nenv: ${JSON.stringify(env)}`);
  for (const dir of [path.dirname(exe), path.resolve(path.dirname(exe), "..")]) {
    console.error(`${dir}: ${fs.readdirSync(dir).slice(0, 100).join(", ")}`);
  }
  process.exit(1);
}

const deadline = Date.now() + 60_000;
while (!fs.existsSync(report) && !exited && Date.now() < deadline) {
  await new Promise((r) => setTimeout(r, 500));
}
await new Promise((r) => setTimeout(r, 500));
child.kill();

if (!fs.existsSync(report)) fail("no report - Python did not run main.py");
const text = fs.readFileSync(report, "utf8");
console.log(text);
const prefix = (/^prefix=(.*)$/m.exec(text)?.[1] ?? "").replace(/^\\\\\?\\/, "");
const errors = [];
if (!prefix.replaceAll("\\", "/").includes("pyembed/python")) errors.push(`not the bundled Python: ${prefix}`);
if (!path.resolve(prefix).startsWith(path.resolve(path.dirname(exe), ".."))) {
  errors.push(`Python is not loaded from the installed app: ${prefix}`);
}
if (/^FAIL/m.test(text)) errors.push("modules failed to import");
if (errors.length) fail(errors.join("; "));
console.log("OK: app runs the bundled Python");
