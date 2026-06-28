// A small, hand-written tokenizer for syntax highlighting — no highlighting
// library. It splits source into whitespace, line comments, string/decimal
// literals, and word tokens (checked against a language's keyword/type
// sets), preserving every character exactly so the rendered block is
// byte-identical to the source.

export type TokenClass = "tok-k" | "tok-s" | "tok-n" | "tok-c" | "tok-t";

export interface Token {
  text: string;
  cls?: TokenClass;
}

const SPLIT_RE = /\/\/[^\n]*|"[^"]*"|\d+(?:\.\d+)?|[A-Za-z_][A-Za-z0-9_]*|\s+|./gs;

export interface TokenizeOptions {
  keywords?: ReadonlySet<string>;
  types?: ReadonlySet<string>;
}

export function tokenize(src: string, opts: TokenizeOptions = {}): Token[] {
  const { keywords, types } = opts;
  const tokens: Token[] = [];
  const re = new RegExp(SPLIT_RE.source, "gs");
  let m: RegExpExecArray | null;
  while ((m = re.exec(src))) {
    const t = m[0];
    if (t.startsWith("//")) {
      tokens.push({ text: t, cls: "tok-c" });
    } else if (t.startsWith('"')) {
      tokens.push({ text: t, cls: "tok-s" });
    } else if (/^\d/.test(t)) {
      tokens.push({ text: t, cls: "tok-n" });
    } else if (keywords?.has(t)) {
      tokens.push({ text: t, cls: "tok-k" });
    } else if (types?.has(t)) {
      tokens.push({ text: t, cls: "tok-t" });
    } else {
      tokens.push({ text: t });
    }
  }
  return tokens;
}

// balanc's full keyword list (src/lex/mod.rs).
export const BAL_KEYWORDS = new Set([
  "txn", "let", "debit", "credit", "currency", "account", "scale", "kind",
  "normal", "rate", "from", "to", "round", "down", "convert", "absorb",
  "split", "split_ratio", "merge",
]);

export const JAVA_KEYWORDS = new Set([
  "package", "public", "private", "static", "final", "class", "new",
  "return", "throw", "throws", "for", "if", "else",
]);

export const JAVA_TYPES = new Set(["int", "long", "boolean", "void", "String"]);
