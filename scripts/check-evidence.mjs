#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const adrDirectory = path.join(repositoryRoot, "docs", "adr.d");
const evidenceDirectory = path.join(repositoryRoot, "docs", "evidence.d");
const staleDays = parseStaleDays(process.argv.slice(2));

function parseStaleDays(argumentsList) {
  if (argumentsList.length === 0) return 90;
  if (argumentsList.length !== 2 || argumentsList[0] !== "--stale-days") {
    throw new Error("usage: node scripts/check-evidence.mjs [--stale-days DAYS]");
  }
  const value = Number(argumentsList[1]);
  if (!Number.isInteger(value) || value < 0) throw new Error("DAYS must be a non-negative integer");
  return value;
}

function read(filePath) {
  return fs.readFileSync(filePath, "utf8");
}

function relative(filePath) {
  return path.relative(repositoryRoot, filePath).split(path.sep).join("/");
}

function recordFiles(directory) {
  return fs.readdirSync(directory)
    .filter((name) => /^\d{4}-.+\.md$/.test(name))
    .sort()
    .map((name) => path.join(directory, name));
}

function acceptedAdrs() {
  return recordFiles(adrDirectory).filter((filePath) => {
    const plainText = read(filePath).replaceAll(/[*_]/g, "");
    return /^- Status:\s*Accepted\s*$/im.test(plainText);
  });
}

function deadheaded() {
  return acceptedAdrs().filter((adrPath) => !fs.existsSync(path.join(evidenceDirectory, path.basename(adrPath))));
}

function orphaned() {
  return recordFiles(evidenceDirectory).filter((evidencePath) => !fs.existsSync(path.join(adrDirectory, path.basename(evidencePath))));
}

function parseValue(rawValue) {
  const value = rawValue.trim();
  if (value.startsWith('"')) return JSON.parse(value);
  if (value.startsWith("'") && value.endsWith("'")) return value.slice(1, -1);
  if (value.startsWith("[") && value.endsWith("]")) {
    if (value === "[]") return [];
    return JSON.parse(value.replaceAll(/'([^']*)'/g, (_, item) => JSON.stringify(item)));
  }
  throw new Error(`unsupported TOML value: ${value}`);
}

function parseChecks(evidencePath) {
  const fence = read(evidencePath).match(/```toml\s*\n([\s\S]*?)\n```/);
  if (!fence) throw new Error("no fenced toml block");

  const document = { check: [] };
  let current = document;
  for (const sourceLine of fence[1].split("\n")) {
    const line = sourceLine.trim();
    if (!line || line.startsWith("#")) continue;
    if (line === "[[check]]") {
      current = {};
      document.check.push(current);
      continue;
    }
    const assignment = line.match(/^([a-z_]+)\s*=\s*(.+)$/i);
    if (!assignment) throw new Error(`unsupported TOML line: ${sourceLine}`);
    current[assignment[1]] = parseValue(assignment[2]);
  }

  const expectedAdr = path.basename(evidencePath, ".md");
  if (document.adr !== expectedAdr) throw new Error(`adr must equal ${JSON.stringify(expectedAdr)}`);
  if (document.check.length === 0) throw new Error("no checks declared");
  return document.check;
}

function wildcardExpression(pattern) {
  let expression = "^";
  for (let index = 0; index < pattern.length; index += 1) {
    const character = pattern[index];
    if (character === "*" && pattern[index + 1] === "*") {
      expression += ".*";
      index += 1;
    } else if (character === "*") {
      expression += "[^/]*";
    } else if (character === "?") {
      expression += "[^/]";
    } else {
      expression += character.replace(/[|\\{}()[\]^$+?.]/g, "\\$&");
    }
  }
  return new RegExp(`${expression}$`);
}

// Directories that are gitignored build/dependency output (see .gitignore):
// never a legitimate source for an evidence pattern, and -- for target/ in
// particular -- subject to concurrent mutation by a running cargo build,
// which can otherwise crash this scan mid-walk with an ENOENT race.
const IGNORED_DIRECTORY_NAMES = new Set([".git", ".mindleak", ".lodestar", "target", "node_modules", "test-results", "playwright-report", "dist"]);

function allFiles(directory) {
  let entries;
  try {
    entries = fs.readdirSync(directory, { withFileTypes: true });
  } catch (error) {
    if (error.code === "ENOENT") return [];
    throw error;
  }
  return entries.flatMap((entry) => {
    const entryPath = path.join(directory, entry.name);
    if (IGNORED_DIRECTORY_NAMES.has(entry.name)) return [];
    return entry.isDirectory() ? allFiles(entryPath) : [entryPath];
  });
}

function resolvePatterns(patterns) {
  if (!Array.isArray(patterns) || patterns.length === 0) return { files: [], missing: ["<no paths>"] };
  const repositoryFiles = allFiles(repositoryRoot);
  const files = [];
  const missing = [];
  for (const pattern of patterns) {
    if (path.isAbsolute(pattern) || pattern.split("/").includes("..")) throw new Error(`path escapes repository: ${pattern}`);
    const matcher = wildcardExpression(pattern);
    const matches = repositoryFiles.filter((filePath) => matcher.test(relative(filePath)));
    if (matches.length === 0) missing.push(pattern);
    else files.push(...matches);
  }
  return { files: [...new Set(files)], missing };
}

function content(filePath) {
  const raw = read(filePath);
  // Markdown emphasis (*_) would otherwise split identifiers like
  // reject_commitment_event_mutation in non-Markdown source/config files.
  return filePath.endsWith(".md") ? raw.replaceAll(/[*_]/g, "") : raw;
}

function runCheck(check) {
  if (!check.id || !check.invariant || !check.type) return ["fail", "id, invariant, and type are required"];
  if (check.type === "parity") {
    const missing = deadheaded().map((filePath) => path.basename(filePath));
    return missing.length ? ["deadhead", `no evidence record for: ${missing.join(", ")}`] : ["pass", "every accepted ADR has a paired evidence record"];
  }
  if (check.type === "manual") {
    if (!check.last_verified) return ["asserted", "never verified"];
    if (!/^\d{4}-\d{2}-\d{2}$/.test(check.last_verified)) return ["fail", `unparseable last_verified: ${check.last_verified}`];
    const verified = new Date(`${check.last_verified}T00:00:00Z`);
    if (Number.isNaN(verified.valueOf())) return ["fail", `unparseable last_verified: ${check.last_verified}`];
    const age = Math.floor((Date.now() - verified.valueOf()) / 86_400_000);
    return age > staleDays ? ["stale", `last verified ${age} days ago, threshold ${staleDays}`] : ["pass", `last verified ${age} days ago`];
  }
  if (check.type !== "present" && check.type !== "absent") return ["fail", `unknown check type: ${check.type}`];
  if (typeof check.pattern !== "string" || check.pattern.length === 0) return ["fail", "pattern must be a non-empty string"];

  const { files, missing } = resolvePatterns(check.paths);
  if (missing.length) return ["fail", `path pattern matched nothing: ${missing.join(", ")}`];
  let expression;
  try {
    expression = new RegExp(check.pattern, "im");
  } catch (error) {
    return ["fail", `invalid pattern: ${error.message}`];
  }
  const matches = files.filter((filePath) => expression.test(content(filePath)));
  if (check.type === "absent") {
    return matches.length ? ["fail", `matched in: ${matches.map(relative).join(", ")}`] : ["pass", `absent from ${files.length} file(s)`];
  }
  const misses = files.filter((filePath) => !matches.includes(filePath));
  return misses.length ? ["fail", `missing from: ${misses.map(relative).join(", ")}`] : ["pass", `present in ${files.length} file(s)`];
}

const stateLabels = [
  ["fail", "BROKEN"],
  ["deadhead", "DEADHEADED"],
  ["stale", "STALE"],
  ["asserted", "ASSERTED"],
];

let broken = false;
console.log(`Evidence check (stale threshold: ${staleDays} days)\n`);
for (const evidencePath of recordFiles(evidenceDirectory)) {
  let checks;
  try {
    checks = parseChecks(evidencePath);
  } catch (error) {
    console.log(`[BROKEN] ${path.basename(evidencePath)}\n         - cannot read checks: ${error.message}\n`);
    broken = true;
    continue;
  }
  const results = checks.map((check) => ({ check, result: runCheck(check) }));
  const statuses = new Set(results.map(({ result }) => result[0]));
  const state = stateLabels.find(([status]) => statuses.has(status))?.[1] ?? "PROVEN";
  if (statuses.has("fail")) broken = true;
  console.log(`[${state}] ${path.basename(evidencePath)}`);
  for (const { check, result } of results) console.log(`         ${result[0].toUpperCase().padEnd(8)} ${check.id ?? "?"} - ${result[1]}`);
  console.log();
}

for (const evidencePath of orphaned()) {
  console.log(`[BROKEN] ${path.basename(evidencePath)}\n         - evidence record has no matching ADR\n`);
  broken = true;
}
for (const adrPath of deadheaded()) console.log(`[DEADHEADED] ${path.basename(adrPath)}\n         - accepted decision has no evidence record\n`);

console.log(broken ? "FAILED: an invariant is violated." : "OK: no invariant is violated.");
process.exitCode = broken ? 1 : 0;