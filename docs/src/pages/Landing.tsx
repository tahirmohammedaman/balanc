import { useId, useState } from "react";
import { Link } from "react-router-dom";
import { NavBar } from "../components/NavBar";
import { Footer } from "../components/Footer";
import { PipelineDiagram } from "../components/PipelineDiagram";
import { ProgramBlock, CodeBlock } from "../components/code/CodeBlock";
import { CopyButton } from "../components/code/CopyButton";
import { C, Rule } from "../components/Prose";
import { program, terminalText, COMPILER_VERSION } from "../data/programs";

const INSTALL =
  "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/tahirmohammedaman/balanc/releases/latest/download/balanc-installer.sh | sh";

interface Folio {
  id: string;
  tab: string;
  path: string;
  verdict: string;
}

/** The hero's three takes on one 45.00 ETB purchase. */
const FOLIOS: Folio[] = [
  {
    id: "twice",
    tab: "Spend it twice",
    path: "examples/coffee_twice.bal",
    verdict: "One credit of 45.00, two debits of the same value. In most ledgers that's a reconciliation ticket. Here it's E_REUSED, and the compiler points at both uses.",
  },
  {
    id: "forget",
    tab: "Forget it",
    path: "examples/coffee_forgotten.bal",
    verdict: "Money leaves assets:cash and goes nowhere. A linear value has to be consumed before the transaction closes, so this is E_DROPPED.",
  },
  {
    id: "split",
    tab: "Split it",
    path: "examples/coffee_split.bal",
    verdict: "split(m, 30.00) consumes m and hands back two values that sum to it. Both are debited once. It compiles, runs, and the trial balance closes.",
  },
];

function LedgerSheet() {
  const [active, setActive] = useState(FOLIOS[0]!.id);
  const base = useId();
  const folio = FOLIOS.find((f) => f.id === active) ?? FOLIOS[0]!;

  function onKey(e: React.KeyboardEvent<HTMLButtonElement>, i: number) {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    const next = FOLIOS[(i + (e.key === "ArrowRight" ? 1 : FOLIOS.length - 1)) % FOLIOS.length]!;
    setActive(next.id);
    document.getElementById(`${base}-tab-${next.id}`)?.focus();
  }

  return (
    <section className="sheet" aria-label="One purchase, three ways">
      <div className="folio-tabs" role="tablist" aria-label="Choose a version of the transaction">
        {FOLIOS.map((f, i) => {
          const ok = program(f.path).exit === 0;
          return (
            <button
              key={f.id}
              id={`${base}-tab-${f.id}`}
              role="tab"
              type="button"
              aria-selected={f.id === active}
              aria-controls={`${base}-panel`}
              tabIndex={f.id === active ? 0 : -1}
              className={`folio-tab ${ok ? "ok" : "err"}`}
              onClick={() => setActive(f.id)}
              onKeyDown={(e) => onKey(e, i)}
            >
              <span className="folio-mark" aria-hidden="true" />
              {f.tab}
            </button>
          );
        })}
      </div>
      <div className="sheet-body" role="tabpanel" id={`${base}-panel`} aria-labelledby={`${base}-tab-${folio.id}`}>
        <ProgramBlock key={folio.path} path={folio.path} />
        <p className="sheet-verdict">{folio.verdict}</p>
      </div>
    </section>
  );
}

interface Invariant {
  rule: string;
  detail: React.ReactNode;
  codes: string[];
}

const INVARIANTS: Invariant[] = [
  {
    rule: "Consumed exactly once",
    detail: (
      <>
        Every <C>Money</C> value bound in a transaction is consumed by exactly one <C>debit</C>, <C>split</C>,{" "}
        <C>split_ratio</C>, <C>merge</C>, or <C>convert</C>. Leftovers and second uses are both type errors.
      </>
    ),
    codes: ["E_DROPPED", "E_REUSED"],
  },
  {
    rule: "Currency is part of the type",
    detail: (
      <>
        <C>Money&lt;USD&gt;</C> cannot reach an ETB account. Crossing currencies takes a declared <C>rate</C>, and{" "}
        <C>convert</C> hands back the rounding residue as a value you must <C>absorb</C>.
      </>
    ),
    codes: ["E_CURRENCY_MISMATCH", "E_EXPECTED_RESIDUE"],
  },
  {
    rule: "Splits conserve the total",
    detail: (
      <>
        <C>split</C> and <C>split_ratio</C> return two values that sum to the input. The one check that needs a
        runtime amount (splitting off more than exists) is made by the evaluator.
      </>
    ),
    codes: ["E_UNBALANCED", "E_ZERO_RATIO"],
  },
  {
    rule: "Amounts fit their currency",
    detail: (
      <>
        A literal can't carry more decimal places than its currency's <C>scale</C>, and there is no negative
        literal to write in the first place.
      </>
    ),
    codes: ["E_TOO_MANY_FRACTION_DIGITS", "E_AMOUNT_OUT_OF_RANGE"],
  },
];

export function Landing() {
  const split = program("examples/coffee_split.bal");
  return (
    <>
      <NavBar variant="landing" />

      <main id="main">
        <header className="hero">
          <div className="wrap">
            <h1>
              Money is a linear resource.
              <span className="h1-sub">Spend it twice and it doesn’t compile.</span>
            </h1>
            <div className="hero-row">
              <p className="lede">
                balanc is a statically typed language for double-entry bookkeeping. Every money value must be
                consumed exactly once, so dropped, duplicated, and silently converted money are compile errors with
                stable codes, not month-end reconciliation tickets.
              </p>
              <div className="hero-actions">
                <Link className="btn btn-primary" to="/docs/tour">
                  Take the language tour
                </Link>
                <Link className="btn btn-secondary" to="/docs/installation">
                  Install
                </Link>
              </div>
            </div>
            <LedgerSheet />
            <p className="provenance">
              Every program and every line of output on this site is produced by running <C>{COMPILER_VERSION}</C>{" "}
              over the files in <C>examples/</C> and <C>tests/cases/</C>. Nothing is retyped by hand.
            </p>
          </div>
        </header>

        <section className="section">
          <div className="wrap">
            <div className="section-head">
              <h2>What the type checker rules out</h2>
              <p>
                Ledger code usually checks these with a test, a reconciliation job, or an alert. In balanc they are
                properties of every program that compiles.
              </p>
            </div>
            <div className="ledger-table" role="table" aria-label="Invariants and their diagnostic codes">
              <div className="lt-row lt-head" role="row">
                <span role="columnheader">Invariant</span>
                <span role="columnheader">What it means</span>
                <span role="columnheader">Rejected with</span>
              </div>
              {INVARIANTS.map((inv) => (
                <div className="lt-row" role="row" key={inv.rule}>
                  <span className="lt-rule" role="cell">
                    {inv.rule}
                  </span>
                  <span className="lt-detail" role="cell">
                    {inv.detail}
                  </span>
                  <span className="lt-codes" role="cell">
                    {inv.codes.map((c) => (
                      <Link key={c} className="err-code" to={`/docs/errors#${c}`}>
                        {c}
                      </Link>
                    ))}
                  </span>
                </div>
              ))}
            </div>
            <p className="section-foot">
              <Link to="/docs/errors">All 21 diagnostic codes, each with a program that triggers it →</Link>
            </p>
          </div>
        </section>

        <section className="section">
          <div className="wrap">
            <div className="section-head">
              <h2>Two ways to read the same program</h2>
            </div>
            <div className="two-readings">
              <div className="reading">
                <h3>If you write type systems</h3>
                <p>
                  The checker threads a linear context Γ through the transaction body. <C>credit</C> introduces a
                  binding, every other operation consumes one, and the body must leave nothing behind.
                </p>
                <Rule premises={"Γ ⊢ e : Money<C>    acct : Account<C>"} name="T-Debit" conclusion={"Γ ⊢ debit(acct, e) : Effect    (consumes e)"} />
                <Rule premises={"Γ ⊢ body ⇒ Δ    Δ = ∅"} name="T-Txn" conclusion={"⊢ txn { body } : Txn"} />
                <p>
                  Currency is a phantom parameter: it exists for the checker and is erased before either backend
                  runs. <Link to="/docs/language/type-system">The full rule set →</Link>
                </p>
              </div>
              <div className="reading">
                <h3>If you keep books</h3>
                <p>
                  <C>credit</C> takes money out of an account; <C>debit</C> puts it into one. The split purchase above
                  posts one credit and two debits, and the interpreter prints the trial balance:
                </p>
                <CodeBlock code={terminalText(split)} lang="out" label={split.command} numbered={false} />
                <p>
                  Accounts carry a declared currency, kind, and normal balance, and an unbalanced entry is not a
                  program you can write. <Link to="/docs/language/core-concepts">Core concepts →</Link>
                </p>
              </div>
            </div>
          </div>
        </section>

        <section className="section">
          <div className="wrap">
            <div className="section-head">
              <h2>One typed IR, two backends</h2>
              <p>
                A zero-dependency Rust front end lowers to a typed IR. A tree-walking interpreter runs it, and a JVM
                backend writes <C>.class</C> bytes by hand, with no ASM and no bytecode library, packaged as a plain{" "}
                <C>.jar</C>.
              </p>
            </div>
            <PipelineDiagram />
            <p className="section-foot">
              <Link to="/docs/architecture/compiler">Compiler architecture →</Link>
              <Link to="/docs/architecture/jvm-backend">Inside the JVM backend →</Link>
            </p>
          </div>
        </section>

        <section className="section install-band">
          <div className="wrap">
            <div className="section-head">
              <h2>Install</h2>
              <p>
                Prebuilt binaries for Linux, macOS, and Windows are on the releases page. On Linux and macOS:
              </p>
            </div>
            <div className="install-line">
              <code>{INSTALL}</code>
              <CopyButton text={INSTALL} />
            </div>
            <p className="section-foot">
              <Link to="/docs/installation">Windows and manual downloads →</Link>
              <Link to="/docs/tour">Then take the tour →</Link>
            </p>
          </div>
        </section>
      </main>

      <Footer />
    </>
  );
}
