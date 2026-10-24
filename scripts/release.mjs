import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { basename, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const targets = {
  "aarch64-apple-darwin": {
    platform: "darwin-aarch64",
    suffix: "macos-arm64",
    folder: "macos",
    extension: ".app.tar.gz",
  },
  "x86_64-apple-darwin": {
    platform: "darwin-x86_64",
    suffix: "macos-x64",
    folder: "macos",
    extension: ".app.tar.gz",
  },
  "x86_64-pc-windows-msvc": {
    platform: "windows-x86_64",
    suffix: "windows-x64",
    folder: "nsis",
    extension: ".exe",
  },
};
const repository = "https://github.com/aagzamov-dev/FlowLink";
const json = (file) => JSON.parse(readFileSync(file, "utf8"));
const writeJson = (file, value) =>
  writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);

export function checkVersions(root = ".") {
  const version = json(join(root, "package.json")).version;
  const cargo = readFileSync(join(root, "Cargo.toml"), "utf8").match(
    /\[workspace\.package\]\s+version\s*=\s*"([^"]+)"/u,
  )?.[1];
  const lock = json(join(root, "package-lock.json"));
  if (
    !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/u.test(version) ||
    [
      cargo,
      json(join(root, "src-tauri/tauri.conf.json")).version,
      lock.version,
      lock.packages[""].version,
    ].some((value) => value !== version)
  ) {
    throw new Error(
      "Use the same stable version in package.json, package-lock.json, Cargo.toml and tauri.conf.json before preparing updates.",
    );
  }
  return version;
}

export function signingConfig(target, mode, env = process.env) {
  if (!targets[target] || !["development", "production"].includes(mode))
    throw new Error("Choose a supported release target and signing mode.");
  if (!env.TAURI_SIGNING_PRIVATE_KEY)
    throw new Error(
      "TAURI_SIGNING_PRIVATE_KEY is required to prepare verified updates. No files will be published.",
    );
  const bundle = { createUpdaterArtifacts: true };
  if (mode === "production" && target.endsWith("apple-darwin")) {
    for (const name of [
      "APPLE_CERTIFICATE",
      "APPLE_CERTIFICATE_PASSWORD",
      "APPLE_SIGNING_IDENTITY",
      "APPLE_ID",
      "APPLE_PASSWORD",
      "APPLE_TEAM_ID",
    ]) {
      if (!env[name]) throw new Error(`Production Mac signing needs ${name}.`);
    }
    if (!env.APPLE_SIGNING_IDENTITY.startsWith("Developer ID Application:"))
      throw new Error(
        "Production Mac builds need a Developer ID Application identity.",
      );
    bundle.macOS = {
      signingIdentity: env.APPLE_SIGNING_IDENTITY,
      hardenedRuntime: true,
    };
  }
  if (mode === "production" && target.endsWith("windows-msvc")) {
    if (!/^[A-Fa-f0-9]{40}$/u.test(env.WINDOWS_CERTIFICATE_THUMBPRINT ?? ""))
      throw new Error(
        "Production Windows signing needs an imported code-signing certificate and WINDOWS_CERTIFICATE_THUMBPRINT.",
      );
    bundle.windows = {
      certificateThumbprint: env.WINDOWS_CERTIFICATE_THUMBPRINT,
      digestAlgorithm: "sha256",
      timestampUrl: "http://timestamp.digicert.com",
      tsp: true,
    };
  }
  return { bundle };
}

export function validateSignature(signature, version, publicKey) {
  const lines = Buffer.from(signature.trim(), "base64")
    .toString("utf8")
    .trim()
    .split(/\r?\n/u);
  const keyLines = Buffer.from(publicKey.trim(), "base64")
    .toString("utf8")
    .trim()
    .split(/\r?\n/u);
  const key = Buffer.from(keyLines[1] ?? "", "base64");
  const sig = Buffer.from(lines[1] ?? "", "base64");
  if (
    lines.length !== 4 ||
    key.length !== 42 ||
    sig.length !== 74 ||
    Buffer.from(lines[3] ?? "", "base64").length !== 64 ||
    !sig.subarray(2, 10).equals(key.subarray(2, 10))
  )
    throw new Error(
      "Update signature is missing, malformed, or belongs to a different signing key.",
    );
  const signedVersion = lines[2]
    .replace(/^trusted comment: /u, "")
    .split("\t")
    .find((field) => field.startsWith("version:"))
    ?.slice(8);
  if (signedVersion !== version)
    throw new Error(
      "Update signature must be bound to the release version. Build with the installed Tauri CLI.",
    );
  return signature.trim();
}

function collect(target) {
  const spec = targets[target];
  if (!spec) throw new Error("Unsupported release target.");
  const version = checkVersions();
  const publicKey = json("src-tauri/tauri.conf.json").plugins.updater.pubkey;
  const bundle = join("target", target, "release/bundle");
  const directory = join(bundle, spec.folder);
  const packages = readdirSync(directory).filter((name) =>
    name.endsWith(spec.extension),
  );
  if (packages.length !== 1)
    throw new Error(
      "Expected one update package. Clean the bundle output before preparing a release.",
    );
  const source = join(directory, packages[0]);
  const signature = validateSignature(
    readFileSync(`${source}.sig`, "utf8"),
    version,
    publicKey,
  );
  const output = join(".tools/release-artifacts", spec.platform);
  mkdirSync(output, { recursive: true });
  const file = `FlowLink_${version}_${spec.suffix}${spec.extension}`;
  copyFileSync(source, join(output, file));
  writeFileSync(join(output, `${file}.sig`), `${signature}\n`);
  if (spec.folder === "macos") {
    const diskImages = readdirSync(join(bundle, "dmg")).filter((name) =>
      name.endsWith(".dmg"),
    );
    if (diskImages.length !== 1)
      throw new Error("Expected one Mac disk image.");
    copyFileSync(
      join(bundle, "dmg", diskImages[0]),
      join(output, `FlowLink_${version}_${spec.suffix}.dmg`),
    );
  }
  writeJson(join(output, `${spec.platform}.json`), {
    version,
    platform: spec.platform,
    file,
    signature,
  });
  console.log(
    `Prepared ${spec.platform} files locally. Nothing was published.`,
  );
}

export function createManifest(
  entries,
  version,
  notes,
  date = new Date().toISOString(),
) {
  const expected = new Set(
    Object.values(targets).map((target) => target.platform),
  );
  const platforms = {};
  for (const entry of entries) {
    if (entry.version !== version || !expected.delete(entry.platform))
      throw new Error(
        "Update manifest has mixed versions, duplicate platforms, or unsupported targets.",
      );
    const spec = Object.values(targets).find(
      (target) => target.platform === entry.platform,
    );
    if (
      entry.file !== `FlowLink_${version}_${spec.suffix}${spec.extension}` ||
      !entry.signature
    )
      throw new Error(
        "Update manifest has an invalid artifact name or missing signature.",
      );
    platforms[entry.platform] = {
      signature: entry.signature,
      url: `${repository}/releases/download/v${version}/${entry.file}`,
    };
  }
  if (expected.size)
    throw new Error(`Missing update builds: ${[...expected].join(", ")}.`);
  return { version, notes, pub_date: date, platforms };
}

function manifest(directory) {
  const version = checkVersions();
  const publicKey = json("src-tauri/tauri.conf.json").plugins.updater.pubkey;
  const entries = Object.values(targets).map(({ platform }) => {
    const entry = json(join(directory, `${platform}.json`));
    validateSignature(entry.signature, version, publicKey);
    if (
      !existsSync(join(directory, entry.file)) ||
      readFileSync(join(directory, `${entry.file}.sig`), "utf8").trim() !==
        entry.signature
    )
      throw new Error(
        "An update package or its matching signature is missing.",
      );
    return entry;
  });
  writeJson(
    join(directory, "latest.json"),
    createManifest(
      entries,
      version,
      readFileSync("docs/RELEASE_NOTES.md", "utf8"),
    ),
  );
  const files = readdirSync(directory)
    .filter(
      (file) => /\.(exe|dmg|gz|sig)$/u.test(file) || file === "latest.json",
    )
    .sort();
  writeFileSync(
    join(directory, "SHA256SUMS.txt"),
    files
      .map(
        (file) =>
          `${createHash("sha256")
            .update(readFileSync(join(directory, file)))
            .digest("hex")}  ${basename(file)}`,
      )
      .join("\n") + "\n",
  );
  console.log(
    "Complete Mac/Windows update feed prepared locally. Nothing was published.",
  );
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  try {
    const [command, target, mode = "development"] = process.argv.slice(2);
    if (command === "check")
      console.log(`Release configuration versions match: ${checkVersions()}`);
    else if (command === "config") {
      checkVersions();
      const config = signingConfig(target, mode);
      mkdirSync(".tools", { recursive: true });
      writeJson(".tools/release-config.json", config);
      console.log(
        `Prepared ${mode} signing configuration. Nothing was published.`,
      );
    } else if (command === "collect") collect(target);
    else if (command === "manifest")
      manifest(target ?? ".tools/release-artifacts");
    else
      throw new Error(
        "Usage: release.mjs check | config <target> <development|production> | collect <target> | manifest <directory>",
      );
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
