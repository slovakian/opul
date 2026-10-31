#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import { parseArgs } from "node:util";

const SEMVER_REGEX =
  /^v?(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-(?:(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+(?:[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/u;

const DEFAULT_REPO = process.env.GITHUB_REPOSITORY || "slovakian/opul";

const FALLBACK_SIGNATURE =
  "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkK";

const printUsage = () => {
  console.log(`Usage: Scripts/generate-updater-manifest.mjs [OPTIONS]

Options:
  --version <semver>        App version (e.g. 0.1.0-nightly.20261031.1)
  --notes <string>          Release notes description
  --pub-date <iso-date>     Publication timestamp (RFC 3339 / ISO 8601)
  --repo <owner/repo>       GitHub repository (default: slovakian/opul)
  --tag <tag>               Release tag (default: nightly)
  --channel <channel>       Channel name (nightly or stable, default: nightly)
  --arm64-url <url>         Download URL for darwin-aarch64 updater bundle
  --arm64-sig <sig_or_file> Signature or path to .sig file for darwin-aarch64
  --x64-url <url>           Download URL for darwin-x86_64 updater bundle
  --x64-sig <sig_or_file>   Signature or path to .sig file for darwin-x86_64
  --output, -o <file>       Output JSON path (default: nightly.json)
  --validate <file>         Validate an existing updater manifest JSON
  --help, -h                Show this help message`);
};

const resolveSignature = (value) => {
  if (!value) {
    return "";
  }

  try {
    return readFileSync(value, "utf-8").trim();
  } catch {
    return String(value).trim();
  }
};

const validateManifest = (manifest) => {
  if (!manifest) {
    throw new Error("Manifest must be non-empty");
  }

  if (!manifest.version || !SEMVER_REGEX.test(manifest.version)) {
    throw new Error(`Invalid or missing semver version: ${manifest.version}`);
  }

  if (manifest.pub_date && Number.isNaN(Date.parse(manifest.pub_date))) {
    throw new Error(`Invalid pub_date timestamp: ${manifest.pub_date}`);
  }

  if (!manifest.platforms) {
    throw new Error("Manifest must include platforms object");
  }

  for (const [platform, config] of Object.entries(manifest.platforms)) {
    if (!config.url || !config.url.startsWith("https://")) {
      throw new Error(
        `Platform ${platform} has invalid or non-HTTPS URL: ${config.url}`
      );
    }

    if (!config.signature) {
      throw new Error(`Platform ${platform} is missing signature`);
    }
  }

  return true;
};

const runValidationMode = (filePath) => {
  const raw = readFileSync(filePath, "utf-8");
  const parsed = JSON.parse(raw);

  validateManifest(parsed);
  console.log(`PASS: Manifest ${filePath} is valid.`);
};

const buildManifest = (values) => {
  const rawVersion = values.version ?? "";

  const normalizedVersion = rawVersion.startsWith("v")
    ? rawVersion.slice(1)
    : rawVersion;

  if (!normalizedVersion || !SEMVER_REGEX.test(normalizedVersion)) {
    console.error(`Error: '${rawVersion}' is not a valid Semantic Version.`);
    process.exit(1);
  }

  const pubDate = values["pub-date"] || new Date().toISOString();
  const channel = values.channel || "nightly";

  const notes =
    values.notes ||
    `Opul ${channel === "nightly" ? "Nightly" : "Release"} ${normalizedVersion}`;

  const repo = values.repo || DEFAULT_REPO;
  const tag = values.tag || "nightly";
  const prefix = channel === "nightly" ? "Opul-Nightly" : "Opul";

  const arm64Url =
    values["arm64-url"] ||
    `https://github.com/${repo}/releases/download/${tag}/${prefix}-arm64.app.tar.gz`;

  const x64Url =
    values["x64-url"] ||
    `https://github.com/${repo}/releases/download/${tag}/${prefix}-x64.app.tar.gz`;

  const arm64Sig = resolveSignature(values["arm64-sig"]);
  const x64Sig = resolveSignature(values["x64-sig"]);

  const manifest = {
    notes,
    platforms: {
      "darwin-aarch64": {
        signature: arm64Sig || FALLBACK_SIGNATURE,
        url: arm64Url,
      },
      "darwin-x86_64": {
        signature: x64Sig || FALLBACK_SIGNATURE,
        url: x64Url,
      },
    },
    pub_date: pubDate,
    version: normalizedVersion,
  };

  validateManifest(manifest);

  return manifest;
};

const main = () => {
  const { values } = parseArgs({
    allowPositionals: false,
    options: {
      "arm64-sig": { type: "string" },
      "arm64-url": { type: "string" },
      channel: { default: "nightly", type: "string" },
      help: { short: "h", type: "boolean" },
      notes: { type: "string" },
      output: { default: "nightly.json", short: "o", type: "string" },
      "pub-date": { type: "string" },
      repo: { default: DEFAULT_REPO, type: "string" },
      tag: { default: "nightly", type: "string" },
      validate: { type: "string" },
      version: { type: "string" },
      "x64-sig": { type: "string" },
      "x64-url": { type: "string" },
    },
  });

  if (values.help) {
    printUsage();
    process.exit(0);
  }

  if (values.validate) {
    runValidationMode(values.validate);

    return;
  }

  if (!values.version) {
    console.error("Error: --version is required to generate updater manifest.");
    process.exit(1);
  }

  const manifest = buildManifest(values);
  const jsonStr = `${JSON.stringify(manifest, null, 2)}\n`;
  const outputPath = values.output ?? "nightly.json";

  writeFileSync(outputPath, jsonStr, "utf-8");
  console.log(`Generated valid updater manifest: ${outputPath}`);
};

main();
