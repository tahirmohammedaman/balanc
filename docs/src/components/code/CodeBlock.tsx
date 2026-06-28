import { Fragment, useMemo } from "react";
import { tokenize, BAL_KEYWORDS, JAVA_KEYWORDS, JAVA_TYPES } from "./tokenize";
import { CopyButton } from "./CopyButton";

export type CodeLang = "bal" | "java" | "ebnf" | "text";

interface CodeBlockProps {
  code: string;
  lang?: CodeLang;
  label?: string;
}

/** A syntax-highlighted `<pre><code>` block. Highlighting is done by a small
 * hand-written tokenizer (see ./tokenize.ts), not a highlighting library. */
export function CodeBlock({ code, lang = "text", label }: CodeBlockProps) {
  const tokens = useMemo(() => {
    switch (lang) {
      case "bal":
        return tokenize(code, { keywords: BAL_KEYWORDS });
      case "java":
        return tokenize(code, { keywords: JAVA_KEYWORDS, types: JAVA_TYPES });
      case "ebnf":
        return tokenize(code, {});
      default:
        return null;
    }
  }, [code, lang]);

  return (
    <>
      {label && <div className="code-label">{label}</div>}
      <div className="code-block">
        <CopyButton text={code} />
        <pre className="code">
          <code>
            {tokens
              ? tokens.map((t, i) =>
                  t.cls ? (
                    <span key={i} className={t.cls}>
                      {t.text}
                    </span>
                  ) : (
                    <Fragment key={i}>{t.text}</Fragment>
                  ),
                )
              : code}
          </code>
        </pre>
      </div>
    </>
  );
}

export interface TermLine {
  type: "cmd" | "out" | "comment";
  text: string;
}

/** A terminal-transcript block: `$ ` command lines, plain output, and `#`
 * comments, each styled distinctly. */
export function Terminal({ lines }: { lines: TermLine[] }) {
  const copyText = lines
    .filter((l) => l.type === "cmd")
    .map((l) => l.text)
    .join("\n");
  return (
    <div className="code-block">
      {copyText && <CopyButton text={copyText} />}
      <pre className="code term">
        <code>
          {lines.map((line, i) => (
            <Fragment key={i}>
              {i > 0 && "\n"}
              {line.type === "cmd" && (
                <>
                  <span className="prompt">$</span> {line.text}
                </>
              )}
              {line.type === "out" && <span className="out">{line.text}</span>}
              {line.type === "comment" && <span className="comment"># {line.text}</span>}
            </Fragment>
          ))}
        </code>
      </pre>
    </div>
  );
}

/** Source | rendered-output panel, used on the landing page's hero. */
export function CodePanel({
  fileLabel,
  code,
  lang,
  outputLabel,
  output,
}: {
  fileLabel: string;
  code: string;
  lang: CodeLang;
  outputLabel: string;
  output: string;
}) {
  const tokens = useMemo(
    () => (lang === "bal" ? tokenize(code, { keywords: BAL_KEYWORDS }) : null),
    [code, lang],
  );
  return (
    <div className="code-panel">
      <div className="code-panel-grid">
        <div className="code-pane code-block">
          <div className="code-pane-head">
            <span className="dots">
              <span />
              <span />
              <span />
            </span>
            {fileLabel}
          </div>
          <CopyButton text={code} />
          <pre>
            <code>
              {tokens
                ? tokens.map((t, i) =>
                    t.cls ? (
                      <span key={i} className={t.cls}>
                        {t.text}
                      </span>
                    ) : (
                      <Fragment key={i}>{t.text}</Fragment>
                    ),
                  )
                : code}
            </code>
          </pre>
        </div>
        <div className="code-pane">
          <div className="code-pane-head">{outputLabel}</div>
          <pre>
            <code>{output}</code>
          </pre>
        </div>
      </div>
    </div>
  );
}
