#!/usr/bin/env node
// Validates docs/features/*.yml and keeps docs/FEATURES.md in sync with it.

import { spawnSync } from "node:child_process";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

import Ajv2020 from "ajv/dist/2020.js";
import { parse } from "yaml";

const root = path.join(import.meta.dirname, "..");
const featureDirectory = path.join(root, "docs/features");
const schemaPath = path.join(root, "docs/features.schema.json");
const markdownPath = path.join(root, "docs/FEATURES.md");

const statusOrder = [
  "shipped",
  "partial",
  "in-progress",
  "planned",
  "idea",
  "wont-do",
];

const statusLegend = [
  ["idea", "Not built. Details say what was checked."],
  ["planned", "Accepted to build, and not started."],
  ["in-progress", "Being built now."],
  ["partial", "Some of it works. Details say what is missing."],
  ["shipped", "Available in the app today."],
  ["wont-do", "Will not be built. A reason is required."],
];

const categoryTitles = {
  "direct-download": "Direct download",
  "media-download": "Media download",
  organization: "Organization",
  other: "Other",
  system: "System",
  torrent: "Torrent",
  ui: "Interface",
};

const schema = JSON.parse(readFileSync(schemaPath, "utf-8"));
const ajv = new Ajv2020({ allErrors: true, strict: true });
const validateFile = ajv.compile(schema);

const calendarDate = (value) => {
  const match = /^(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})$/u.exec(value);

  if (!match) {
    return false;
  }

  const year = Number(match.groups.year);
  const month = Number(match.groups.month);
  const day = Number(match.groups.day);
  const date = new Date(Date.UTC(year, month - 1, day));

  return (
    date.getUTCFullYear() === year &&
    date.getUTCMonth() === month - 1 &&
    date.getUTCDate() === day
  );
};

const featureErrors = (feature, where) => {
  const errors = [];

  if (!calendarDate(feature.added)) {
    errors.push(`${where}: added is not a real date`);
  }

  if (!calendarDate(feature.updated)) {
    errors.push(`${where}: updated is not a real date`);
  }

  const { reason, status } = feature;

  const missingReason =
    status === "wont-do" &&
    (reason === null || reason === undefined || String(reason).trim() === "");

  if (missingReason) {
    errors.push(`${where}: wont-do requires a reason`);
  }

  return errors;
};

const featureRecord = (value) => {
  if (!value || Array.isArray(value)) {
    return null;
  }

  const record = new Object(value);

  if (!("status" in record)) {
    return null;
  }

  return record;
};

const parseFeatureFile = (name, text) => {
  let document;

  try {
    document = parse(text, { schema: "core" });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);

    return { errors: [`${name}: ${message}`], features: [] };
  }

  const errors = [];

  if (!validateFile(document)) {
    for (const error of validateFile.errors ?? []) {
      const location = error.instancePath || "(file)";
      errors.push(`${name}${location}: ${error.message ?? "invalid"}`);
    }
  }

  const category = name.replace(/\.yml$/u, "");
  const features = Array.isArray(document?.features) ? document.features : [];

  if (!schema.$defs.feature.properties.category.enum.includes(category)) {
    errors.push(`${name}: file name is not a feature category`);
  }

  for (const feature of features) {
    const record = featureRecord(feature);

    if (!record) {
      continue;
    }

    const where = `${name}#${record.id ?? "?"}`;

    if (record.category !== category) {
      errors.push(`${where}: category must match ${category}.yml`);
    }

    errors.push(...featureErrors(record, where));
  }

  return { errors, features: errors.length === 0 ? features : [] };
};

const duplicateErrors = (features) => {
  const seen = new Map();
  const errors = [];

  for (const feature of features) {
    const previous = seen.get(feature.id);

    if (previous) {
      errors.push(
        `duplicate id ${feature.id} in ${previous} and ${feature.category}.yml`
      );
    } else {
      seen.set(feature.id, `${feature.category}.yml`);
    }
  }

  return errors;
};

const loadTracker = () => {
  const names = readdirSync(featureDirectory)
    .filter((name) => name.endsWith(".yml"))
    .toSorted();

  const errors = [];
  const features = [];

  if (names.length === 0) {
    errors.push("docs/features has no YAML files");
  }

  for (const name of names) {
    const text = readFileSync(path.join(featureDirectory, name), "utf-8");
    const parsed = parseFeatureFile(name, text);
    errors.push(...parsed.errors);
    features.push(...parsed.features);
  }

  errors.push(...duplicateErrors(features));

  return { errors, features };
};

const cell = (value) =>
  String(value).replaceAll("|", "\\|").replaceAll("\n", " ");

const byStatusThenTitle = (left, right) => {
  const status =
    statusOrder.indexOf(left.status) - statusOrder.indexOf(right.status);

  if (status !== 0) {
    return status;
  }

  return left.title.localeCompare(right.title);
};

const counts = (features) =>
  statusOrder.map((status) => [
    status,
    features.filter((feature) => feature.status === status).length,
  ]);

const renderMarkdown = (features) => {
  const lines = [
    "<!-- This file is generated from docs/features by Scripts/check-features.mjs. Do not edit. -->",
    "",
    "# Opul features",
    "",
    "This list is generated from [`docs/features/`](features/). Edit the YAML, then run `node Scripts/check-features.mjs --write`.",
    "",
    "How a feature moves from an idea to shipped is in [feature workflow](feature-workflow.md).",
    "",
    "## Status legend",
    "",
    "| Status | Meaning |",
    "| --- | --- |",
    ...statusLegend.map(
      ([status, meaning]) => `| \`${status}\` | ${meaning} |`
    ),
    "",
    "Priority is `p0` through `p3`. A dash means no priority is set.",
    "",
    "## Counts",
    "",
    "| Status | Count |",
    "| --- | ---: |",
    ...counts(features).map(
      ([status, count]) => `| \`${status}\` | ${count} |`
    ),
    `| **Total** | **${features.length}** |`,
    "",
  ];

  for (const category of schema.$defs.feature.properties.category.enum) {
    const group = features
      .filter((feature) => feature.category === category)
      .toSorted(byStatusThenTitle);

    if (group.length === 0) {
      continue;
    }

    lines.push(
      `## ${categoryTitles[category]}`,
      "",
      "| ID | Title | Status | Priority | Source | Summary |",
      "| --- | --- | --- | --- | --- | --- |"
    );

    for (const feature of group) {
      const priority = feature.priority ?? "—";
      lines.push(
        `| \`${cell(feature.id)}\` | ${cell(feature.title)} | \`${feature.status}\` | ${priority} | \`${cell(feature.source)}\` | ${cell(feature.summary)} |`
      );
    }

    const detailed = group.filter(
      (feature) =>
        feature.notes.trim() !== "" ||
        feature.status === "wont-do" ||
        feature.status === "partial" ||
        feature.status === "idea"
    );

    if (detailed.length > 0) {
      lines.push("", "### Details", "");

      for (const feature of detailed) {
        const reason =
          feature.status === "wont-do" ? ` Reason: ${feature.reason}` : "";

        lines.push(
          `- **${feature.title}** (\`${feature.id}\`) — ${feature.notes}${reason}`
        );
      }
    }

    lines.push("");
  }

  return `${lines.join("\n").replace(/\n+$/u, "")}\n`;
};

// Match `vp fmt` so the committed catalog stays formatted.
const formatMarkdown = (markdown) => {
  const formatter = path.join(root, "node_modules/.bin/vp");

  const result = spawnSync(
    formatter,
    ["fmt", "--stdin-filepath", "docs/FEATURES.md"],
    {
      cwd: root,
      encoding: "utf-8",
      input: markdown,
    }
  );

  if (result.status !== 0) {
    const detail = result.stderr || result.stdout || "formatter failed";
    throw new Error(detail);
  }

  return result.stdout;
};

const sample = (overrides = {}) => ({
  added: "2026-11-17",
  category: "other",
  id: "sample-feature",
  issue: null,
  notes: "Not a product feature.",
  priority: null,
  reason: null,
  source: "opul-original",
  status: "idea",
  summary: "A sample entry used only by the validator self-test.",
  title: "Sample feature",
  updated: "2026-11-17",
  ...overrides,
});

const assertIncludes = (errors, needle) => {
  if (!errors.some((error) => error.includes(needle))) {
    throw new Error(`expected an error containing ${JSON.stringify(needle)}`);
  }
};

const selfTest = () => {
  const valid = parseFeatureFile(
    "other.yml",
    `features:\n  - id: sample-feature\n    title: Sample feature\n    category: other\n    source: competitor:downie\n    status: wont-do\n    priority: p2\n    summary: A sample entry used only by the validator self-test.\n    notes: Not a product feature.\n    reason: The self-test needs a reason.\n    issue: null\n    added: 2026-11-17\n    updated: 2026-11-17\n`
  );

  if (valid.errors.length > 0) {
    throw new Error(`valid sample failed: ${valid.errors.join("; ")}`);
  }

  const duplicate = duplicateErrors([
    sample({ category: "torrent", id: "same-id" }),
    sample({ category: "ui", id: "same-id" }),
  ]);

  assertIncludes(duplicate, "duplicate id same-id");

  const refused = parseFeatureFile(
    "other.yml",
    'features:\n  - id: missing-reason\n    title: Missing reason\n    category: other\n    source: qbittorrent\n    status: wont-do\n    priority: null\n    summary: A rejected idea still needs to say why it was rejected.\n    notes: ""\n    reason: null\n    issue: null\n    added: 2026-11-17\n    updated: 2026-11-17\n'
  );

  assertIncludes(refused.errors, "wont-do requires a reason");

  const badDate = featureErrors(
    sample({ added: "2026-02-31" }),
    "memory#sample-feature"
  );

  assertIncludes(badDate, "added is not a real date");

  const badId = parseFeatureFile(
    "other.yml",
    'features:\n  - id: Not Kebab\n    title: Bad id\n    category: other\n    source: yt-dlp\n    status: idea\n    priority: p4\n    summary: This entry is intentionally invalid for the self-test.\n    notes: ""\n    reason: null\n    issue: https://example.com/not-an-issue\n    added: 2026-11-17\n    updated: 2026-11-17\n'
  );

  if (badId.errors.length === 0) {
    throw new Error("invalid sample was accepted");
  }

  const mismatched = parseFeatureFile(
    "torrent.yml",
    'features:\n  - id: wrong-file\n    title: Wrong file\n    category: ui\n    source: opul-original\n    status: idea\n    priority: null\n    summary: This entry claims a category that does not match its file.\n    notes: ""\n    reason: null\n    issue: null\n    added: 2026-11-17\n    updated: 2026-11-17\n'
  );

  assertIncludes(mismatched.errors, "category must match torrent.yml");
};

const usage = `Usage: node Scripts/check-features.mjs --check|--write

--check  Fail when the tracker is invalid or docs/FEATURES.md is stale.
--write  Regenerate docs/FEATURES.md from docs/features/.
`;

const main = () => {
  selfTest();
  const [mode] = process.argv.slice(2);

  if (mode !== "--check" && mode !== "--write") {
    process.stderr.write(usage);
    process.exitCode = 1;

    return;
  }

  const { errors, features } = loadTracker();

  if (errors.length > 0) {
    process.stderr.write(`${errors.join("\n")}\n`);
    process.exitCode = 1;

    return;
  }

  const markdown = formatMarkdown(renderMarkdown(features));

  if (mode === "--write") {
    writeFileSync(markdownPath, markdown);
    process.stdout.write(
      `Wrote ${features.length} features to docs/FEATURES.md\n`
    );

    return;
  }

  const current = readFileSync(markdownPath, "utf-8");

  if (current !== markdown) {
    process.stderr.write(
      "docs/FEATURES.md is out of date. Run node Scripts/check-features.mjs --write\n"
    );
    process.exitCode = 1;

    return;
  }

  const summary = counts(features)
    .map(([status, count]) => `${status}=${count}`)
    .join(" ");

  process.stdout.write(`Feature tracker ok (${features.length}: ${summary})\n`);
};

main();
