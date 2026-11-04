# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Two audiences, weighted equally (confirmed):

- **Compiler and PL engineers** evaluating the idea. They want the typing rules, the
  invariants, and evidence that the diagnostics are real compiler output, not mock-ups.
- **Fintech / ledger engineers** evaluating whether the language prevents real
  bookkeeping bugs (vanished money, double-spent money, unbalanced entries, silent
  currency mixing) and how to install and run it.

## Product Purpose

balanc is a statically typed language for double-entry bookkeeping in which ledger
invariants are enforced by the type system instead of checked at runtime. The docs site
(balanc.tahiraman.me, source in `docs/`) explains the idea, teaches the language, and is
the reference for its CLI, diagnostics, and backends. Success: a reader understands
"money as a linear resource" from the landing page alone, and can find any error code's
meaning and fix in one click.

## Positioning

Money is linear: every money value must be consumed exactly once. Dropping or reusing
it, mixing currencies without an explicit `Rate`, or splitting off more than exists is a
compile-time (or, for `E_UNBALANCED`, evaluation-time) error with a stable code. The JVM
backend hand-writes `.class` bytes in Rust with no bytecode library.

## Operating Context

Readers arrive from GitHub, links shared in PL/fintech circles, and search. They copy
CLI commands, compare code to compiler output, and look up `E_*` codes after hitting
one in a terminal.

## Capabilities and Constraints

- Compiler: Rust, zero dependencies. Tree-walking interpreter + JVM bytecode backend
  over one typed IR. CLI: `balanc [--json] [--emit-jvm <out.class>] <file.bal>`.
- 21 stable diagnostic codes in `src/diag/codes.rs`; each has a fixed `= note:` line.
- Docs stack (confirmed): React + TypeScript + Vite SPA in `docs/`, deployed to
  Cloudflare Workers with SPA fallback; must stay buildable on its own.
- Undecided: browser playground (WASM) is planned, not built.

## Brand Commitments

- Name is lowercase `balanc`. Files use `.bal`.
- Voice (confirmed): precise, technical, a little playful. Claims must be exact; jokes
  never replace a fact.

## Evidence on Hand

- `examples/coffee.bal` and `tests/cases/*.bal` with snapshot output in
  `tests/snapshots/*.txt`. All code/output shown on the site must come from running
  these through the compiler. Do not invent diagnostics, benchmarks, users, or adoption.
- No testimonials, customers, or performance numbers exist.

## Product Principles

1. Show, then claim: every assertion about the type system is backed by real source and
   real compiler output.
2. Errors are a feature: diagnostics are first-class content, catalogued and linkable.
3. Precision over persuasion: exact terminology (linear, consumed, residue, scale).
4. Serve both readers: the PL reader gets rules; the ledger reader gets bugs prevented.
