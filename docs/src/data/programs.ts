// Typed access to outputs.generated.json — the real compiler's output for
// every program under examples/ and tests/cases/. Regenerate with
// `node scripts/gen-outputs.mjs` after `cargo build`; CI fails if it is stale.

import data from "./outputs.generated.json";

export interface Program {
  path: string;
  source: string;
  command: string;
  exit: number;
  stdout: string;
  stderr: string;
  json: string | null;
  codes: string[];
}

type RawProgram = Omit<Program, "path">;
const PROGRAMS = data.programs as Record<string, RawProgram>;

/** The `balanc --version` string the outputs were generated with. */
export const COMPILER_VERSION: string = data.compiler;

/** Every stable diagnostic code declared in src/diag/codes.rs, in order. */
export const DECLARED_CODES: string[] = data.codes;

export function program(path: string): Program {
  const p = PROGRAMS[path];
  if (!p) throw new Error(`No generated output for ${path} — run scripts/gen-outputs.mjs`);
  return { path, ...p };
}

/** What a terminal shows: stdout then stderr, without the trailing newline. */
export function terminalText(p: Program): string {
  return (p.stdout + p.stderr).replace(/\n+$/, "");
}

/** 1-based source lines a diagnostic points at, from its `--> file:L:C` lines. */
export function flaggedLines(p: Program): { primary: Set<number>; secondary: Set<number> } {
  const primary = new Set<number>();
  const secondary = new Set<number>();
  let target = primary;
  for (const line of p.stderr.split("\n")) {
    if (/^error\[/.test(line)) target = primary;
    else if (/^note:/.test(line)) target = secondary;
    const m = line.match(/-->\s+\S+?:(\d+):\d+/);
    if (m && m[1]) target.add(Number(m[1]));
  }
  for (const n of primary) secondary.delete(n);
  return { primary, secondary };
}
