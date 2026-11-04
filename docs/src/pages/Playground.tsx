import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { NavBar } from "../components/NavBar";
import { Footer } from "../components/Footer";
import { Lines } from "../components/code/CodeBlock";
import { CopyButton } from "../components/code/CopyButton";
import { program } from "../data/programs";
import { CompilerMissingError, loadCompiler, type Compiler, type JsonResult } from "../playground/compiler";

const FILE = "playground.bal";

/** Programs the editor can be reset to; the first is the default. */
const EXAMPLES = [
  { path: "examples/coffee_twice.bal", label: "coffee_twice.bal: spend it twice" },
  { path: "examples/coffee_forgotten.bal", label: "coffee_forgotten.bal: forget it" },
  { path: "examples/coffee_split.bal", label: "coffee_split.bal: split it" },
  { path: "examples/coffee.bal", label: "coffee.bal: a balanced day" },
] as const;

type View = "text" | "json";

interface Run {
  /** The source this run compiled, to tell when the editor has moved on. */
  source: string;
  text: string;
  json: string;
  result: JsonResult;
}

type Status =
  | { kind: "loading" }
  | { kind: "ready" }
  | { kind: "missing" }
  | { kind: "failed"; message: string };

function linesFlaggedBy(result: JsonResult): { primary: Set<number>; secondary: Set<number> } {
  const primary = new Set<number>();
  const secondary = new Set<number>();
  if (!result.ok) {
    for (const d of result.diagnostics) {
      primary.add(d.line);
      for (const s of d.secondary ?? []) secondary.add(s.line);
    }
  }
  for (const n of primary) secondary.delete(n);
  return { primary, secondary };
}

/** A textarea laid exactly over the ruled, numbered, highlighted lines that
 * ProgramBlock draws, so typing keeps the ledger look and flagged lines. */
function Editor({
  value,
  onChange,
  onSubmit,
  primary,
  secondary,
  labelledBy,
}: {
  value: string;
  onChange: (v: string) => void;
  onSubmit: () => void;
  primary?: Set<number>;
  secondary?: Set<number>;
  labelledBy: string;
}) {
  return (
    <div className="pg-editor">
      <Lines code={value} lang="bal" numbered primary={primary} secondary={secondary} />
      <textarea
        aria-labelledby={labelledBy}
        aria-describedby="pg-shortcut"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            onSubmit();
          }
        }}
        spellCheck={false}
        autoCapitalize="off"
        autoComplete="off"
        autoCorrect="off"
        wrap="off"
      />
    </div>
  );
}

export function Playground() {
  const [source, setSource] = useState(() => program(EXAMPLES[0].path).source);
  const [status, setStatus] = useState<Status>({ kind: "loading" });
  const [run, setRun] = useState<Run | null>(null);
  const [crash, setCrash] = useState<string | null>(null);
  const [view, setView] = useState<View>("text");
  const compiler = useRef<Compiler | null>(null);
  const ids = useId();
  const srcLabel = `${ids}-src`;

  const load = useCallback(() => {
    setStatus({ kind: "loading" });
    loadCompiler().then(
      (c) => {
        compiler.current = c;
        setStatus({ kind: "ready" });
      },
      (e: unknown) => {
        if (e instanceof CompilerMissingError) setStatus({ kind: "missing" });
        else setStatus({ kind: "failed", message: e instanceof Error ? e.message : String(e) });
      },
    );
  }, []);

  useEffect(load, [load]);

  const typeCheck = useCallback(() => {
    const c = compiler.current;
    if (!c) return;
    try {
      const json = c.check(source);
      const text = c.check_pretty(source);
      setRun({ source, text, json, result: JSON.parse(json) as JsonResult });
      setCrash(null);
    } catch (e) {
      setRun(null);
      setCrash(e instanceof Error ? e.message : String(e));
    }
  }, [source]);

  const stale = run !== null && run.source !== source;
  const flags = useMemo(() => (run && !stale ? linesFlaggedBy(run.result) : undefined), [run, stale]);
  const ok = run?.result.ok ?? false;
  const shown = run ? (view === "json" ? run.json : run.text).replace(/\n+$/, "") : "";
  const ready = status.kind === "ready";

  return (
    <>
      <NavBar variant="docs" />
      <main className="wrap pg" id="main">
        <header className="pg-head">
          <h1>Playground</h1>
          <p className="doc-lede">
            Type-check a balanc program in your browser. The compiler runs here as WebAssembly, built from the same
            Rust source as the CLI, and prints exactly what <code>balanc {FILE}</code> would. Nothing is sent anywhere.
          </p>
        </header>

        <div className="pg-grid">
          <section className="code-block pg-src" aria-labelledby={srcLabel}>
            <div className="code-head">
              <span className="path" id={srcLabel}>
                {FILE}
              </span>
              <label className="pg-example">
                <span className="sr-only">Load an example</span>
                <select
                  value=""
                  onChange={(e) => {
                    if (e.target.value) setSource(program(e.target.value).source);
                  }}
                >
                  <option value="">Load example…</option>
                  {EXAMPLES.map((ex) => (
                    <option key={ex.path} value={ex.path}>
                      {ex.label}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <CopyButton text={source} />
            <Editor
              value={source}
              onChange={setSource}
              onSubmit={typeCheck}
              primary={flags?.primary}
              secondary={flags?.secondary}
              labelledBy={srcLabel}
            />
            <div className="pg-actions">
              <button type="button" className="btn btn-primary" onClick={typeCheck} disabled={!ready}>
                {status.kind === "loading" ? "Loading compiler…" : "Type-check"}
              </button>
              <span className="pg-hint" id="pg-shortcut">
                or <kbd>Ctrl</kbd> + <kbd>Enter</kbd> in the editor
              </span>
            </div>
          </section>

          <section
            className={`code-block program-out pg-out${run ? (ok ? " is-ok" : " is-err") : ""}${stale ? " is-stale" : ""}`}
            aria-label="Compiler output"
          >
            <div className="code-head">
              <span className="cmd">
                <span className="prompt" aria-hidden="true">
                  $
                </span>{" "}
                balanc {view === "json" ? "--json " : ""}
                {FILE}
              </span>
              {run && <span className={`exit ${ok ? "ok" : "err"}`}>exit {ok ? 0 : 1}</span>}
            </div>
            <div className="pg-out-bar">
              <div className="seg" role="group" aria-label="Output format">
                <button type="button" aria-pressed={view === "text"} onClick={() => setView("text")}>
                  Text
                </button>
                <button type="button" aria-pressed={view === "json"} onClick={() => setView("json")}>
                  --json
                </button>
              </div>
              {stale && <span className="pg-stale">Edited since this run</span>}
            </div>
            <div aria-live="polite">
              {run ? (
                <>
                  <span className="sr-only">
                    {run.result.ok
                      ? "Accepted: the program type-checks and the trial balance closes."
                      : `Rejected with ${run.result.diagnostics.map((d) => d.code).join(", ")}.`}
                  </span>
                  <CopyButton text={view === "json" ? run.json : run.text} />
                  <Lines code={shown} lang={view === "json" ? "text" : "out"} />
                </>
              ) : (
                <p className={`pg-empty${status.kind === "failed" || status.kind === "missing" || crash ? " is-error" : ""}`}>
                  {crash ? (
                    <>The compiler stopped with an internal error on this input ({crash}). That's a bug in balanc; please report it with the program on GitHub.</>
                  ) : status.kind === "missing" ? (
                    <>
                      This build of the site doesn't include the WebAssembly compiler. Run <code>playground/build.sh</code>,
                      then rebuild the site.
                    </>
                  ) : status.kind === "failed" ? (
                    <>
                      The compiler didn't load ({status.message}).{" "}
                      <button type="button" className="linklike" onClick={load}>
                        Try again
                      </button>
                    </>
                  ) : (
                    <>Press Type-check to run the compiler on {FILE}. Its diagnostics or trial balance print here.</>
                  )}
                </p>
              )}
            </div>
          </section>
        </div>

        <p className="pg-foot">
          Look up any <code>E_*</code> code in the <Link to="/docs/errors">error catalogue</Link>, or take the{" "}
          <Link to="/docs/tour">tour</Link> for the language one program at a time.
        </p>
      </main>
      <Footer />
    </>
  );
}
