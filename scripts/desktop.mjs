import { existsSync } from "node:fs";
import { spawn } from "node:child_process";
import { resolve, delimiter } from "node:path";
const env = { ...process.env };
for (const key of [
  "APPLE_ID",
  "APPLE_PASSWORD",
  "APPLE_TEAM_ID",
  "APPLE_CERTIFICATE",
  "APPLE_CERTIFICATE_PASSWORD",
  "APPLE_SIGNING_IDENTITY",
]) {
  if (env[key] === "") delete env[key];
}
const localCargo = resolve(".tools/cargo");
if (existsSync(resolve(localCargo, "bin/cargo"))) {
  env.CARGO_HOME = localCargo;
  env.RUSTUP_HOME = resolve(".tools/rustup");
  env.PATH = `${resolve(localCargo, "bin")}${delimiter}${env.PATH ?? ""}`;
}
const cli = resolve("node_modules/@tauri-apps/cli/tauri.js");
const child = spawn(process.execPath, [cli, ...process.argv.slice(2)], {
  env,
  stdio: "inherit",
});
child.on("exit", (code) => process.exit(code ?? 1));
