import hljs from "highlight.js/lib/core";
import java from "highlight.js/lib/languages/java";
import ebnf from "highlight.js/lib/languages/ebnf";
import bash from "highlight.js/lib/languages/bash";
import { balanc, balancOutput } from "./hljs-balanc";

hljs.registerLanguage("balanc", balanc);
hljs.registerLanguage("balanc-output", balancOutput);
hljs.registerLanguage("java", java);
hljs.registerLanguage("ebnf", ebnf);
hljs.registerLanguage("bash", bash);

export type CodeLang = "bal" | "out" | "java" | "ebnf" | "bash" | "text";

const HLJS_NAME: Record<Exclude<CodeLang, "text">, string> = {
  bal: "balanc",
  out: "balanc-output",
  java: "java",
  ebnf: "ebnf",
  bash: "bash",
};

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** Highlighted HTML for `code` (highlight.js escapes its input). */
export function highlight(code: string, lang: CodeLang): string {
  if (lang === "text") return escapeHtml(code);
  return hljs.highlight(code, { language: HLJS_NAME[lang], ignoreIllegals: true }).value;
}

/**
 * Splits highlighted HTML into one string per source line, closing any span
 * still open at a line break and reopening it on the next line, so each line
 * can be wrapped in its own element (for line numbers and flagged lines).
 */
export function splitHighlightedLines(html: string): string[] {
  const lines: string[] = [];
  const open: string[] = [];
  let current = "";
  const re = /(<span[^>]*>)|(<\/span>)|(\n)|([^<\n]+|<)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(html))) {
    if (m[1]) {
      open.push(m[1]);
      current += m[1];
    } else if (m[2]) {
      open.pop();
      current += m[2];
    } else if (m[3]) {
      current += "</span>".repeat(open.length);
      lines.push(current);
      current = open.join("");
    } else {
      current += m[4];
    }
  }
  lines.push(current);
  return lines;
}
