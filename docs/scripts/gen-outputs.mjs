#!/usr/bin/env node
// Runs every .bal program under examples/ and tests/cases/ through the real
// balanc compiler and records its source, output, and exit code in
// src/data/outputs.generated.json. Every code sample and every diagnostic
// shown on the site is read from that file, so nothing on the site is
// hand-typed compiler output.
//
//   cargo build                      # from the repo root
//   node scripts/gen-outputs.mjs     # from docs/
//
// BALANC=/path/to/balanc overrides the binary (default: ../target/debug/balanc).
// CI runs this and fails if the committed JSON is stale.

import { spawnSync } from "node:child_process";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, relative, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const docsDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(docsDir, "..");
const bin = process.env.BALANC ?? join(repoRoot, "target", "debug", "balanc");

function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else if (name.endsWith(".bal")) out.push(p);
  }
  return out;
}

const files = [...walk(join(repoRoot, "examples")), ...walk(join(repoRoot, "tests", "cases"))];

const version = spawnSync(bin, ["--version"], { encoding: "utf8" });
if (version.status !== 0) {
  console.error(`gen-outputs: cannot run ${bin} — build the compiler first (cargo build)`);
  process.exit(1);
}

const programs = {};
for (const abs of files) {
  const rel = relative(repoRoot, abs).split("\\").join("/");
  // Run from the repo root with a repo-relative path so diagnostics print the
  // same `--> examples/...` paths a reader would see.
  const r = spawnSync(bin, [rel], { cwd: repoRoot, encoding: "utf8" });
  const j = spawnSync(bin, ["--json", rel], { cwd: repoRoot, encoding: "utf8" });
  const codes = [...new Set([...(r.stderr ?? "").matchAll(/^error\[(E_[A-Z_]+)\]/gm)].map((m) => m[1]))];
  programs[rel] = {
    source: readFileSync(abs, "utf8"),
    command: `balanc ${rel}`,
    exit: r.status,
    stdout: r.stdout,
    stderr: r.stderr,
    json: r.status === 0 ? null : j.stdout,
    codes,
  };
}

// Every stable code in src/diag/codes.rs must be produced by at least one
// program, so the error catalogue never has an entry without real output.
const declared = [...readFileSync(join(repoRoot, "src", "diag", "codes.rs"), "utf8").matchAll(/=> "(E_[A-Z_]+)"/g)].map(
  (m) => m[1],
);
const produced = new Set(Object.values(programs).flatMap((p) => p.codes));
const missing = declared.filter((c) => !produced.has(c));
if (missing.length) {
  console.error(`gen-outputs: no example program produces ${missing.join(", ")} — add one under examples/errors/`);
  process.exit(1);
}

const outPath = join(docsDir, "src", "data", "outputs.generated.json");
writeFileSync(
  outPath,
  JSON.stringify({ compiler: version.stdout.trim(), codes: declared, programs }, null, 2) + "\n",
);
console.log(`gen-outputs: wrote ${Object.keys(programs).length} programs to ${relative(docsDir, outPath)}`);
