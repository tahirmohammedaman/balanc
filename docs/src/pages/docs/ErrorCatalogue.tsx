import { useEffect, useMemo, useState } from "react";
import { Link, useLocation } from "react-router-dom";
import { DocPage } from "../../components/DocPage";
import { C } from "../../components/Prose";
import { CodeBlock, ProgramBlock } from "../../components/code/CodeBlock";
import { DECLARED_CODES, program } from "../../data/programs";

type Phase = "lex" | "parse" | "resolve" | "typeck" | "eval";

interface Entry {
  code: string;
  phase: Phase;
  title: string;
  meaning: React.ReactNode;
  example: string;
  clip?: boolean;
}

// Meanings follow the doc comments on each variant in src/diag/codes.rs; the
// phase is the compiler pass that emits the code. Examples are real files.
const ENTRIES: Entry[] = [
  {
    code: "E_LEX_UNEXPECTED_CHAR",
    phase: "lex",
    title: "Unexpected character",
    meaning: (
      <>
        A character does not begin any valid token. Comments are <C>//</C>; there is no <C>#</C>. The lexer has
        no error recovery, so a file reports at most one lex error.
      </>
    ),
    example: "examples/errors/lex_unexpected_char.bal",
  },
  {
    code: "E_PARSE_UNEXPECTED_TOKEN",
    phase: "parse",
    title: "Unexpected token",
    meaning: <>The parser found a token it cannot use at this point in the grammar, most often a missing <C>;</C>.</>,
    example: "examples/errors/parse_unexpected_token.bal",
  },
  {
    code: "E_PARSE_UNEXPECTED_EOF",
    phase: "parse",
    title: "Unexpected end of file",
    meaning: <>The file ended while a construct such as <C>{"txn { … }"}</C> was still open.</>,
    example: "examples/errors/parse_unexpected_eof.bal",
  },
  {
    code: "E_EXPR_TOO_DEEP",
    phase: "parse",
    title: "Expression too deep",
    meaning: (
      <>
        A <C>merge</C>/<C>split</C>/<C>split_ratio</C> chain nests more than 256 levels. It is rejected before the
        recursive-descent parser can overflow its stack.
      </>
    ),
    example: "examples/errors/expr_too_deep.bal",
    clip: true,
  },
  {
    code: "E_UNBOUND_NAME",
    phase: "resolve",
    title: "Unbound name",
    meaning: <>A name has no <C>let</C> binding in scope.</>,
    example: "examples/errors/unbound_name.bal",
  },
  {
    code: "E_UNKNOWN_CURRENCY",
    phase: "resolve",
    title: "Unknown currency",
    meaning: <>An <C>account</C> names a currency with no matching <C>currency</C> declaration.</>,
    example: "tests/cases/unknown_currency.bal",
  },
  {
    code: "E_DUPLICATE_CURRENCY",
    phase: "resolve",
    title: "Duplicate currency",
    meaning: <>Two <C>currency</C> declarations use the same name.</>,
    example: "examples/errors/duplicate_currency.bal",
  },
  {
    code: "E_DUPLICATE_ACCOUNT",
    phase: "resolve",
    title: "Duplicate account",
    meaning: <>Two <C>account</C> declarations name the same account path.</>,
    example: "examples/errors/duplicate_account.bal",
  },
  {
    code: "E_UNDECLARED_ACCOUNT",
    phase: "resolve",
    title: "Undeclared account",
    meaning: (
      <>
        A <C>credit</C> or <C>debit</C> names an account path with no <C>account</C> declaration. Every account must
        resolve to a declared currency before types can be inferred.
      </>
    ),
    example: "tests/cases/undeclared_account.bal",
  },
  {
    code: "E_UNKNOWN_ACCOUNT_KIND",
    phase: "resolve",
    title: "Unknown account kind",
    meaning: (
      <>
        <C>kind</C> must be one of <C>asset</C>, <C>liability</C>, <C>equity</C>, <C>income</C>, <C>expense</C>.
      </>
    ),
    example: "examples/errors/unknown_account_kind.bal",
  },
  {
    code: "E_UNDECLARED_RATE",
    phase: "resolve",
    title: "Undeclared rate",
    meaning: <>A <C>convert</C> or <C>absorb</C> names a rate with no matching <C>rate</C> declaration.</>,
    example: "examples/errors/undeclared_rate.bal",
  },
  {
    code: "E_DUPLICATE_RATE",
    phase: "resolve",
    title: "Duplicate rate",
    meaning: <>Two <C>rate</C> declarations use the same name.</>,
    example: "examples/errors/duplicate_rate.bal",
  },
  {
    code: "E_TOO_MANY_FRACTION_DIGITS",
    phase: "typeck",
    title: "Too many fraction digits",
    meaning: (
      <>
        A decimal literal has more fractional digits than its currency’s <C>scale</C>. Checked once the literal’s
        currency is known, since scale is per currency.
      </>
    ),
    example: "tests/cases/too_many_fraction_digits.bal",
  },
  {
    code: "E_AMOUNT_OUT_OF_RANGE",
    phase: "typeck",
    title: "Amount out of range",
    meaning: <>A literal is too large for the fixed-point <C>i64</C> representation amounts are stored in.</>,
    example: "tests/cases/amount_out_of_range.bal",
  },
  {
    code: "E_CURRENCY_MISMATCH",
    phase: "typeck",
    title: "Currency mismatch",
    meaning: (
      <>
        Money of one currency meets an account, rate, or residue of another. The secondary note shows where the
        value’s currency was established. Use <C>convert</C> to cross currencies.
      </>
    ),
    example: "tests/cases/currency_mismatch.bal",
  },
  {
    code: "E_EXPECTED_MONEY",
    phase: "typeck",
    title: "Expected money, found residue",
    meaning: <>A <C>debit</C> was given a conversion residue. Residues are discharged with <C>absorb</C>.</>,
    example: "tests/cases/expected_money_found_residue.bal",
  },
  {
    code: "E_EXPECTED_RESIDUE",
    phase: "typeck",
    title: "Expected residue, found money",
    meaning: <>An <C>absorb</C> was given ordinary money. Money is discharged with <C>debit</C>.</>,
    example: "tests/cases/expected_residue_found_money.bal",
  },
  {
    code: "E_DROPPED",
    phase: "typeck",
    title: "Money dropped",
    meaning: (
      <>
        A money value or residue is still unconsumed when the transaction body ends: the linear context Δ is not
        empty.
      </>
    ),
    example: "examples/coffee_forgotten.bal",
  },
  {
    code: "E_REUSED",
    phase: "typeck",
    title: "Money reused",
    meaning: <>A money value is consumed a second time. The note points at the first use.</>,
    example: "examples/coffee_twice.bal",
  },
  {
    code: "E_ZERO_RATIO",
    phase: "typeck",
    title: "Zero ratio",
    meaning: <><C>split_ratio(e, p, q)</C> with <C>p = q = 0</C> doesn’t determine an allocation.</>,
    example: "tests/cases/zero_ratio.bal",
  },
  {
    code: "E_UNBALANCED",
    phase: "eval",
    title: "Unbalanced split",
    meaning: (
      <>
        <C>split(e, n)</C> asks for more than <C>e</C> holds. The amount of <C>e</C> isn’t known to the type
        checker, so this is the one invariant checked by the evaluator.
      </>
    ),
    example: "tests/cases/split_unbalanced.bal",
  },
];

const PHASES: { id: Phase | "all"; label: string }[] = [
  { id: "all", label: "All" },
  { id: "lex", label: "Lex" },
  { id: "parse", label: "Parse" },
  { id: "resolve", label: "Resolve" },
  { id: "typeck", label: "Type check" },
  { id: "eval", label: "Evaluate" },
];

/** The note line for a code, as the compiler printed it. */
function noteFor(e: Entry): string {
  const p = program(e.example);
  const lines = p.stderr.split("\n");
  const start = lines.findIndex((l) => l.startsWith(`error[${e.code}]`));
  const note = lines.slice(start).find((l) => l.trim().startsWith("= note:"));
  return note ? note.trim().replace(/^= note:\s*/, "") : "";
}

export function ErrorCatalogue() {
  const [query, setQuery] = useState("");
  const [phase, setPhase] = useState<Phase | "all">("all");
  const { hash } = useLocation();

  const missing = DECLARED_CODES.filter((c) => !ENTRIES.some((e) => e.code === c));
  if (missing.length) throw new Error(`Error catalogue is missing ${missing.join(", ")}`);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return ENTRIES.filter(
      (e) =>
        (phase === "all" || e.phase === phase) &&
        (!q || e.code.toLowerCase().includes(q) || e.title.toLowerCase().includes(q) || noteFor(e).toLowerCase().includes(q)),
    );
  }, [query, phase]);

  // Deep links (/docs/errors#E_REUSED) scroll once the entry is rendered.
  useEffect(() => {
    if (!hash) return;
    // Wait a frame so ScrollToTop and layout settle before jumping.
    const id = requestAnimationFrame(() => {
      document.getElementById(decodeURIComponent(hash.slice(1)))?.scrollIntoView();
    });
    return () => cancelAnimationFrame(id);
  }, [hash]);

  const json = program("examples/coffee_forgotten.bal").json ?? "";

  return (
    <DocPage
      title="Error Catalogue"
      lede="Every diagnostic balanc can emit, with a program that triggers it and the compiler’s real output. Codes are stable: once released, a code is never reused for a different meaning."
    >
      <h2 id="reading">Reading a diagnostic</h2>
      <p>
        Diagnostics are rendered rustc-style: a header with the code and message, the source line with carets under
        the exact span, and a fixed <C>= note:</C> line of guidance that is the same for every occurrence of that
        code. Some add a secondary <C>note:</C> with a second span, such as where a value was first consumed. A file
        with several independent errors reports all of them, sorted by position. With <C>--json</C>, the same
        information is printed to stdout:
      </p>
      <CodeBlock
        code={JSON.stringify(JSON.parse(json), null, 2)}
        lang="text"
        label="balanc --json examples/coffee_forgotten.bal  (indented for reading)"
        numbered={false}
      />

      <h2 id="catalogue">Catalogue</h2>
      <div className="catalogue-tools">
        <label className="field">
          <span className="field-label">Filter</span>
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Code, name, or note text"
          />
        </label>
        <div className="seg" role="group" aria-label="Compiler phase">
          {PHASES.map((p) => (
            <button key={p.id} type="button" aria-pressed={phase === p.id} onClick={() => setPhase(p.id)}>
              {p.label}
            </button>
          ))}
        </div>
      </div>

      <table className="code-index">
        <caption className="sr-only">Diagnostic codes</caption>
        <thead>
          <tr>
            <th scope="col">Code</th>
            <th scope="col">Phase</th>
            <th scope="col">Meaning</th>
          </tr>
        </thead>
        <tbody>
          {shown.map((e) => (
            <tr key={e.code}>
              <td>
                <a className="err-code" href={`#${e.code}`}>
                  {e.code}
                </a>
              </td>
              <td className="phase">{e.phase}</td>
              <td>{e.title}</td>
            </tr>
          ))}
          {shown.length === 0 && (
            <tr>
              <td colSpan={3} className="empty">
                No code matches “{query}”.{" "}
                <button type="button" className="linklike" onClick={() => { setQuery(""); setPhase("all"); }}>
                  Clear filters
                </button>
              </td>
            </tr>
          )}
        </tbody>
      </table>

      {shown.map((e) => (
        <section className="err-entry" key={e.code} aria-labelledby={e.code}>
          <h3 id={e.code}>
            <span className="err-code">{e.code}</span>
            <span className="err-title">{e.title}</span>
          </h3>
          <dl className="err-facts">
            <div>
              <dt>Phase</dt>
              <dd>{e.phase}</dd>
            </div>
            <div>
              <dt>Note</dt>
              <dd>{noteFor(e)}</dd>
            </div>
          </dl>
          <p>{e.meaning}</p>
          <ProgramBlock path={e.example} clip={e.clip} />
        </section>
      ))}

      <p>
        Each code is declared once in <C>src/diag/codes.rs</C>. The <Link to="/docs/tour">language tour</Link> walks
        through the most common ones in context.
      </p>
    </DocPage>
  );
}
