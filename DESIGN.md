# Design

The balanc docs site (`docs/`) is set like an accountant's columnar ledger and
read like standard documentation. Tokens live in `docs/src/styles/app.css`.

## World

- **Ground** — faint green ledger paper (`--ground`), with `--paper` /
  `--paper-2` for code, panels, and headers.
- **Rules** — green hairlines (`--rule`), a stronger rule (`--rule-strong`) for
  frames, and a **double rule** (`3px double`) under page titles, the hero, and
  the last row of every table: the bookkeeper's "this total is final".
- **Ink** — green-black `--ink`, with `--ink-2` (secondary) and `--ink-3`
  (meta; still ≥4.5:1 on the ground).
- **Green** (`--green`) is the one action colour: links, primary buttons,
  active nav, selected filters.
- **Red ink** (`--red`) is reserved for errors and negative amounts, as in a
  ledger. `--red-rule` draws the red double margin rule between line numbers
  and code, and the left rule of the "rejected with" column.
- Dark theme is the same ledger at night; both themes follow the system and
  can be overridden by the toggle.

## Type

- Public Sans for all text and headings (700–800 for display, tight tracking).
- JetBrains Mono for code, error codes, paths, and figures only.

## Code surfaces

- Every code line sits on a ledger hairline (`--line` pitch).
- `.bal` source shows line numbers left of a red double margin rule.
- `ProgramBlock` pairs a source file with the command, the compiler's real
  output, and an `exit N` stamp. Lines a diagnostic points at are washed red
  (primary) or amber (secondary note).
- Syntax colours are `--syn-*` tokens; highlight.js scopes map to them. Output
  uses `hljs-diag-*` scopes.

## Components

- **Ledger sheet** (landing hero): folio tabs over a `ProgramBlock`; a dot marks
  each tab as failing (red ring) or passing (filled green). Output lines print
  in with a short clip-path reveal, disabled under reduced motion.
- **Ledger table**: rule column, explanation, and a red-ruled "amount" column of
  linked error codes.
- **Inference rule**: premises over a 1.5px ink bar, rule name in green at the
  right end.
- Docs chrome: sticky sidebar with filter, right-rail contents, prev/next under
  a double rule. Radii 6px; one soft shadow, used only on the hero sheet.

## Content rule

No compiler output is typed by hand. It comes from
`docs/src/data/outputs.generated.json` (see `docs/README.md`).
