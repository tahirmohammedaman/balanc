// highlight.js grammars for balanc.
//
//   balanc             — `.bal` source. Token set mirrors src/lex/mod.rs.
//   balanc-output      — what the `balanc` CLI prints: rustc-style diagnostics
//                        (error[CODE], --> location, gutter, ^^^ carets,
//                        = note:) and the trial-balance report.
//
// Both are plain highlight.js `LanguageFn`s, so they work outside this site:
//
//   import hljs from "highlight.js/lib/core";
//   import { balanc } from "./hljs-balanc";
//   hljs.registerLanguage("balanc", balanc);
//
// Scopes that highlight.js themes already style (keyword, built_in, type,
// number, string, comment, symbol, literal) are used for source. The output
// grammar uses `diag-*` scopes, emitted as `hljs-diag-*` classes.

import type { HLJSApi, Language } from "highlight.js";

/** Keywords that structure a program (src/lex/mod.rs). */
const DECLARATION_KEYWORDS = [
  "txn", "let", "currency", "account", "rate", "from", "to", "round", "down",
  "scale", "kind", "normal",
];

/** The seven total-preserving money operations — also lexer keywords. */
const MONEY_OPERATIONS = [
  "credit", "debit", "convert", "absorb", "split", "split_ratio", "merge",
];

/** The closed set of account kinds checked by E_UNKNOWN_ACCOUNT_KIND. */
const ACCOUNT_KINDS = ["asset", "liability", "equity", "income", "expense"];

export function balanc(hljs: HLJSApi): Language {
  return {
    name: "balanc",
    aliases: ["bal"],
    case_insensitive: false,
    keywords: {
      $pattern: "[A-Za-z_][A-Za-z0-9_]*",
      keyword: DECLARATION_KEYWORDS,
      built_in: MONEY_OPERATIONS,
      literal: ACCOUNT_KINDS,
    },
    contains: [
      hljs.C_LINE_COMMENT_MODE,
      // String literals have no escapes in balanc: the lexer stops at the next `"`.
      { scope: "string", begin: /"/, end: /"/ },
      // Account paths: `assets:cash`, `income:fx_rounding`.
      {
        scope: "symbol",
        match: /\b[a-z_][a-z0-9_]*(?::[a-z_][a-z0-9_]*)+\b/,
      },
      // Currency names are conventionally ISO 4217 codes: ETB, USD, JPY.
      { scope: "type", match: /\b[A-Z][A-Z0-9_]*\b/ },
      // Decimal literals. There is no `-` in literal position (D-014).
      { scope: "number", match: /\b\d+(?:\.\d+)?\b/ },
    ],
  };
}

export function balancOutput(): Language {
  return {
    name: "balanc output",
    aliases: ["bal-out"],
    contains: [
      // error[E_DROPPED]: this money value is never consumed
      {
        begin: [/^error/, /\[/, /E_[A-Z_]+/, /\]/],
        beginScope: { 1: "diag-error", 3: "diag-code" },
      },
      // note: first consumed here   (secondary label)
      { scope: "diag-label", match: /^note(?=:)/ },
      //   = note: fixed guidance for the code
      { scope: "diag-note", begin: /^\s*= note:/, end: /$/ },
      //   --> file.bal:6:7
      {
        begin: [/^\s*-->\s/, /\S+/],
        beginScope: { 1: "diag-gutter", 2: "diag-location" },
      },
      // Carets under the offending span.
      { scope: "diag-caret", match: /\^+/ },
      // The `NN |` gutter.
      { scope: "diag-gutter", match: /^\s*\d*\s*\|/ },
      // Trial Balance (ETB)
      { scope: "diag-heading", match: /^Trial Balance \([A-Z0-9_]+\)/ },
      // Debit total / Credit total rows.
      { scope: "diag-total", match: /^\s+(?:Debit|Credit) total/ },
      // Negative and positive amounts in the report.
      { scope: "diag-neg", match: /-\d+\.\d+\b/ },
      { scope: "number", match: /\b\d+\.\d+\b/ },
      { scope: "symbol", match: /\b[a-z_][a-z0-9_]*(?::[a-z_][a-z0-9_]*)+\b/ },
    ],
  };
}
