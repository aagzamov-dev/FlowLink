import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  checkVersions,
  createManifest,
  signingConfig,
  targets,
  validateSignature,
} from "./release.mjs";

test("release versions agree across manifests", () => {
  assert.equal(
    checkVersions(),
    JSON.parse(readFileSync("package.json", "utf8")).version,
  );
});

test("production packaging requires real signing credentials", () => {
  assert.throws(
    () => signingConfig("aarch64-apple-darwin", "production", {}),
    /TAURI_SIGNING_PRIVATE_KEY/u,
  );
  const env = { TAURI_SIGNING_PRIVATE_KEY: "fixture" };
  assert.throws(
    () => signingConfig("aarch64-apple-darwin", "production", env),
    /APPLE_CERTIFICATE/u,
  );
  assert.throws(
    () => signingConfig("x86_64-pc-windows-msvc", "production", env),
    /WINDOWS_CERTIFICATE_THUMBPRINT/u,
  );
  assert.deepEqual(signingConfig("aarch64-apple-darwin", "development", env), {
    bundle: { createUpdaterArtifacts: true },
  });
});

test("signature metadata must match the signing key and version", () => {
  const signature = readFileSync(
    "src-tauri/tests/fixtures/update.txt.sig",
    "utf8",
  );
  const publicKey = readFileSync("src-tauri/tests/fixtures/update.pub", "utf8");
  assert.equal(
    validateSignature(signature, "0.2.1", publicKey),
    signature.trim(),
  );
  assert.throws(
    () => validateSignature(signature, "0.2.2", publicKey),
    /release version/u,
  );
  assert.throws(
    () =>
      validateSignature(
        signature,
        "0.2.1",
        JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")).plugins
          .updater.pubkey,
      ),
    /different signing key/u,
  );
  assert.throws(
    () => validateSignature("broken", "0.2.1", publicKey),
    /malformed/u,
  );
});

const entries = Object.values(targets).map((target) => ({
  version: "0.2.1",
  platform: target.platform,
  file: `FlowLink_0.2.1_${target.suffix}${target.extension}`,
  signature: "fixture",
}));
test("manifest includes both Mac architectures and Windows with version-pinned URLs", () => {
  const manifest = createManifest(
    entries,
    "0.2.1",
    "Update both devices.",
    "2026-10-24T00:00:00Z",
  );
  assert.equal(Object.keys(manifest.platforms).length, 3);
  assert.match(
    manifest.platforms["darwin-aarch64"].url,
    /\/v0\.2\.1\/FlowLink_0\.2\.1_macos-arm64\.app\.tar\.gz$/u,
  );
  assert.match(manifest.platforms["windows-x86_64"].url, /\.exe$/u);
});
test("incomplete, duplicate, mismatched or path-traversing packages cannot become an update feed", () => {
  assert.throws(
    () => createManifest(entries.slice(1), "0.2.1", ""),
    /Missing update builds/u,
  );
  assert.throws(
    () => createManifest([...entries, entries[0]], "0.2.1", ""),
    /duplicate/u,
  );
  assert.throws(() => createManifest(entries, "0.2.2", ""), /mixed versions/u);
  assert.throws(
    () =>
      createManifest(
        [{ ...entries[0], file: "../bad.exe" }, ...entries.slice(1)],
        "0.2.1",
        "",
      ),
    /invalid artifact/u,
  );
});
