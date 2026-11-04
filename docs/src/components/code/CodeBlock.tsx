import { Fragment, useMemo } from "react";
import { highlight, splitHighlightedLines, type CodeLang } from "./highlight";
import { CopyButton } from "./CopyButton";
import { flaggedLines, program, terminalText } from "../../data/programs";

export type { CodeLang };

/** Lines longer than this are clipped for display (only E_EXPR_TOO_DEEP's
 * 260-deep `merge` chain needs it). The copy button still copies the full text. */
const CLIP_AT = 110;

function clipLines(text: string, clip: boolean): { text: string; clipped: boolean } {
  if (!clip) return { text, clipped: false };
  let clipped = false;
  const out = text
    .split("\n")
    .map((l) => {
      if (l.length <= CLIP_AT) return l;
      clipped = true;
      return l.slice(0, CLIP_AT) + " …";
    })
    .join("\n");
  return { text: out, clipped };
}

export interface LinesProps {
  code: string;
  lang: CodeLang;
  numbered?: boolean;
  primary?: Set<number>;
  secondary?: Set<number>;
}

/** The ruled body of every code surface: one row per line, optional
 * line-number gutter, and flagged lines (where a diagnostic points). */
export function Lines({ code, lang, numbered = false, primary, secondary }: LinesProps) {
  const lines = useMemo(() => splitHighlightedLines(highlight(code, lang)), [code, lang]);
  return (
    <pre className={`code${numbered ? " numbered" : ""}`} data-lang={lang}>
      <code>
        {lines.map((html, i) => {
          const n = i + 1;
          const flag = primary?.has(n) ? " flag" : secondary?.has(n) ? " flag-2" : "";
          return (
            <span className={`ln${flag}`} key={i}>
              {numbered && (
                <span className="ln-no" aria-hidden="true">
                  {n}
                </span>
              )}
              <span className="ln-text" dangerouslySetInnerHTML={{ __html: html || " " }} />
              {"\n"}
            </span>
          );
        })}
      </code>
    </pre>
  );
}

interface CodeBlockProps {
  code: string;
  lang?: CodeLang;
  label?: string;
  numbered?: boolean;
}

/** A highlighted code block (highlight.js, with the balanc grammar in ./hljs-balanc.ts). */
export function CodeBlock({ code, lang = "text", label, numbered }: CodeBlockProps) {
  return (
    <figure className="code-block">
      {label && <figcaption className="code-head">{label}</figcaption>}
      <CopyButton text={code} />
      <Lines code={code} lang={lang} numbered={numbered ?? lang === "bal"} />
    </figure>
  );
}

export interface TermLine {
  type: "cmd" | "out" | "comment";
  text: string;
}

/** A terminal transcript: `$` command lines, compiler output, `#` comments. */
export function Terminal({ lines }: { lines: TermLine[] }) {
  const copyText = lines
    .filter((l) => l.type === "cmd")
    .map((l) => l.text)
    .join("\n");
  return (
    <figure className="code-block term">
      {copyText && <CopyButton text={copyText} />}
      <pre className="code">
        <code>
          {lines.map((line, i) => (
            <Fragment key={i}>
              {i > 0 && "\n"}
              {line.type === "cmd" && (
                <>
                  <span className="prompt" aria-hidden="true">
                    $
                  </span>{" "}
                  <span dangerouslySetInnerHTML={{ __html: highlight(line.text, "bash") }} />
                </>
              )}
              {line.type === "out" && (
                <span className="out" dangerouslySetInnerHTML={{ __html: highlight(line.text, "out") }} />
              )}
              {line.type === "comment" && <span className="comment"># {line.text}</span>}
            </Fragment>
          ))}
        </code>
      </pre>
    </figure>
  );
}

/** Real compiler output for a terminal transcript, read from the generated data. */
export function RealOutput({ path, prefix = "balanc" }: { path: string; prefix?: string }) {
  const p = program(path);
  return (
    <Terminal
      lines={[
        { type: "cmd", text: `${prefix} ${path}` },
        { type: "out", text: terminalText(p) },
      ]}
    />
  );
}

interface ProgramBlockProps {
  /** Repo-relative path, e.g. "examples/coffee.bal". */
  path: string;
  /** Hide the source and show only the command and its output. */
  outputOnly?: boolean;
  /** Clip very long lines for display. */
  clip?: boolean;
}

/**
 * A program and what the real compiler printed for it: the source (with the
 * lines a diagnostic points at flagged), then the command, its output, and
 * its exit status. Everything comes from outputs.generated.json.
 */
export function ProgramBlock({ path, outputOnly = false, clip = false }: ProgramBlockProps) {
  const p = program(path);
  const { primary, secondary } = flaggedLines(p);
  const src = clipLines(p.source.replace(/\n$/, ""), clip);
  const out = clipLines(terminalText(p), clip);
  const ok = p.exit === 0;
  return (
    <figure className={`program ${ok ? "is-ok" : "is-err"}`}>
      {!outputOnly && (
        <div className="program-src code-block">
          <figcaption className="code-head">
            <span className="path">{path}</span>
          </figcaption>
          <CopyButton text={p.source} />
          <Lines code={src.text} lang="bal" numbered primary={primary} secondary={secondary} />
        </div>
      )}
      <div className="program-out code-block">
        <div className="code-head">
          <span className="cmd">
            <span className="prompt" aria-hidden="true">
              $
            </span>{" "}
            {p.command}
          </span>
          <span className={`exit ${ok ? "ok" : "err"}`}>exit {p.exit}</span>
        </div>
        <Lines code={out.text} lang="out" />
      </div>
      {(src.clipped || out.clipped) && (
        <p className="program-note">Lines longer than {CLIP_AT} characters are clipped here; the file in the repo is complete.</p>
      )}
    </figure>
  );
}
