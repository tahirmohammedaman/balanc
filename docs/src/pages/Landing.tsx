import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { NavBar } from "../components/NavBar";
import { Footer } from "../components/Footer";
import { PipelineDiagram } from "../components/PipelineDiagram";
import { CodePanel } from "../components/code/CodeBlock";
import { C } from "../components/Prose";

const COFFEE_SRC = `currency ETB { scale = 2 }

account assets:cash { currency = ETB, kind = asset, normal = debit }
account expenses:coffee { currency = ETB, kind = expense, normal = debit }

txn "coffee" {
  let m = credit(assets:cash, 45.00);
  debit(expenses:coffee, m);
}`;

const COFFEE_OUTPUT = `Trial Balance (ETB)
  assets:cash                    -45.00
  expenses:coffee                 45.00
  Debit total                      0.00
  Credit total                     0.00`;

function FeatureIcon({ path }: { path: ReactNode }) {
  return (
    <div className="icon">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        {path}
      </svg>
    </div>
  );
}

function FeatureCard({ icon, title, children }: { icon: ReactNode; title: string; children: ReactNode }) {
  return (
    <div className="card">
      <FeatureIcon path={icon} />
      <h3>{title}</h3>
      <p>{children}</p>
    </div>
  );
}

function FrameRow({ term, children }: { term: string; children: ReactNode }) {
  return (
    <div className="frame-row">
      <span className="term">{term}</span>
      <p>{children}</p>
    </div>
  );
}

function OpTourCard({
  name,
  tone,
  children,
}: {
  name: string;
  tone: "green" | "red" | "blue";
  children: ReactNode;
}) {
  return (
    <div className="card op-tour-card">
      <span className={`badge badge-${tone}`}>{name}</span>
      <p style={{ marginTop: 12 }}>{children}</p>
    </div>
  );
}

export function Landing() {
  return (
    <>
      <NavBar variant="landing" />

      <header className="hero">
        <div className="wrap">
          <span className="eyebrow">
            <span className="dot" /> interpreter + JVM backend, differentially tested to agree
          </span>
          <h1>
            The ledger balances
            <br />
            because it <span className="hl">has to</span> — not because someone checked.
          </h1>
          <p className="lede">
            balanc is a statically typed language for double-entry bookkeeping. Money is a <strong>linear</strong>{" "}
            value: every unit must be consumed exactly once. Dropping it, duplicating it, or writing a
            transaction whose debits and credits disagree isn't a bug you find later — it's a compile error.
          </p>
          <div className="hero-ctas">
            <Link className="btn btn-primary" to="/docs/getting-started">
              Get started →
            </Link>
            <Link className="btn btn-secondary" to="/docs/language/core-concepts">
              Read the language guide
            </Link>
          </div>

          <CodePanel
            fileLabel="examples/coffee.bal"
            code={COFFEE_SRC}
            lang="bal"
            outputLabel="$ cargo run -- examples/coffee.bal"
            output={COFFEE_OUTPUT}
          />

          <div className="stat-strip">
            <div className="stat">
              <b>7</b>
              <span>money operations</span>
            </div>
            <div className="stat">
              <b>21</b>
              <span>stable diagnostic codes</span>
            </div>
            <div className="stat">
              <b>2</b>
              <span>backends, one typed IR</span>
            </div>
            <div className="stat">
              <b>0</b>
              <span>runtime dependencies</span>
            </div>
          </div>
        </div>
      </header>

      <section className="section">
        <div className="wrap">
          <div className="section-head">
            <div className="section-kicker">Why balanc</div>
            <h2>Accounting bugs, moved from runtime to compile time</h2>
            <p>
              Most ledger software checks balance with a post-hoc assertion — a test, a reconciliation job, a
              monitoring alert. balanc makes the check structural: there is no program you can write in which
              money silently appears, vanishes, or gets counted twice.
            </p>
          </div>
          <div className="grid grid-4">
            <FeatureCard
              title="Money is linear"
              icon={
                <>
                  <path d="M9 12l2 2 4-4" />
                  <circle cx="12" cy="12" r="9" />
                </>
              }
            >
              Every <C>Money</C> value must be consumed exactly once. Drop it and you get <C>E_DROPPED</C>; reuse
              it and you get <C>E_REUSED</C> — both are compile-time diagnostics, not production incidents.
            </FeatureCard>
            <FeatureCard
              title="Currency is a type"
              icon={
                <>
                  <rect x="3" y="3" width="8" height="8" rx="1.5" />
                  <rect x="13" y="13" width="8" height="8" rx="1.5" />
                  <path d="M11 7h6a2 2 0 0 1 2 2v4M13 17H7a2 2 0 0 1-2-2v-4" />
                </>
              }
            >
              <C>Money&lt;ETB&gt;</C> and <C>Money&lt;USD&gt;</C> are distinct types that can't be mixed. Crossing
              currencies takes an explicit <C>rate</C> and produces an accounted-for rounding <C>Residue</C> —
              never a silent truncation.
            </FeatureCard>
            <FeatureCard
              title="Two backends, one truth"
              icon={
                <>
                  <rect x="4" y="4" width="16" height="16" rx="2" />
                  <path d="M4 10h16M10 4v16" />
                </>
              }
            >
              The same typed program runs on a tree-walking interpreter or compiles to a hand-written JVM{" "}
              <C>.class</C> file. Both are differentially tested to produce byte-identical trial balances.
            </FeatureCard>
            <FeatureCard
              title="Exact, never float"
              icon={
                <>
                  <path d="M12 3v18M5 8l7-5 7 5M5 8v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8" />
                </>
              }
            >
              Amounts are fixed-point integers in the currency's minor unit. <C>f64</C> can't represent 0.10
              exactly, and this language's whole premise depends on arithmetic that can't drift.
            </FeatureCard>
          </div>
        </div>
      </section>

      <section className="section">
        <div className="wrap">
          <div className="section-head">
            <div className="section-kicker">Two audiences, one language</div>
            <h2>Reads like a type system. Balances like a ledger.</h2>
            <p>
              balanc's constructs map directly onto ordinary double-entry bookkeeping — the type checker is just
              enforcing rules an accountant already relies on, made explicit enough for a compiler to verify.
            </p>
          </div>
          <div className="frame-cols">
            <div className="frame-col dev">
              <span className="frame-tag">FOR DEVELOPERS</span>
              <FrameRow term="Money<C>">
                A linear value, parameterized by currency — think an affine/linear type, checked the same way a
                borrow checker tracks ownership.
              </FrameRow>
              <FrameRow term="Γ ⊢ e : T">
                The type context is literally the set of live, not-yet-consumed bindings in the current
                transaction body.
              </FrameRow>
              <FrameRow term="Δ = ∅">
                "No dropped values" is the closing premise of every transaction — the compiler proves it, not a
                test suite.
              </FrameRow>
              <FrameRow term="TModule">
                One typed IR feeds both an interpreter and a hand-written JVM bytecode emitter — no separate
                "compiler" and "runtime" model to keep in sync.
              </FrameRow>
            </div>
            <div className="frame-col fin">
              <span className="frame-tag">FOR FINTECH &amp; ACCOUNTING</span>
              <FrameRow term="Money<C>">
                A unit of currency that exists in exactly one place at a time — you can move it, split it, or
                convert it, but never copy it or lose track of it.
              </FrameRow>
              <FrameRow term="credit / debit">
                The same double-entry primitives from any ledger — <C>credit</C> takes money out of an account,{" "}
                <C>debit</C> puts it into one.
              </FrameRow>
              <FrameRow term="unbalanced txn">
                A transaction where debits and credits don't agree simply doesn't compile — there's no
                "unbalanced" state a ledger can ever be left in.
              </FrameRow>
              <FrameRow term="trial balance">
                Running a program produces the same report a bookkeeper would draw up by hand: accounts grouped
                by kind, debit and credit subtotals.
              </FrameRow>
            </div>
          </div>
        </div>
      </section>

      <section className="section">
        <div className="wrap">
          <div className="section-head">
            <div className="section-kicker">Under the hood</div>
            <h2>One typed IR, two ways to run it</h2>
            <p>
              Source text passes through six stages to a typed module (<C>TModule</C>), which the tree-walking
              interpreter evaluates directly and the JVM backend compiles to bytecode — nothing type-related
              lives downstream of <C>typeck</C> in either path.
            </p>
          </div>
          <PipelineDiagram />
          <p style={{ marginTop: 16, fontSize: 13.5 }}>
            See the full <Link to="/docs/architecture/compiler">compiler architecture</Link> and{" "}
            <Link to="/docs/architecture/jvm-backend">JVM backend</Link> writeups for how each stage works.
          </p>
        </div>
      </section>

      <section className="section">
        <div className="wrap">
          <div className="section-head">
            <div className="section-kicker">Seven operations, closed set</div>
            <h2>Every way to move money, in one screenful</h2>
            <p>
              There is no escape hatch — no unsafe block, no runtime cast. If a program type-checks, it was built
              entirely from these seven operations.
            </p>
          </div>
          <div className="grid grid-4">
            <OpTourCard name="credit" tone="green">
              Takes money out of an account, producing a fresh <C>Money&lt;C&gt;</C>.
            </OpTourCard>
            <OpTourCard name="debit" tone="red">
              Puts money into an account, consuming it.
            </OpTourCard>
            <OpTourCard name="convert" tone="blue">
              Crosses currencies via a declared <C>rate</C>, returning the converted amount plus a{" "}
              <C>Residue</C>.
            </OpTourCard>
            <OpTourCard name="absorb" tone="red">
              Discharges a <C>Residue</C> into an account — the only thing that can consume one.
            </OpTourCard>
            <OpTourCard name="split" tone="blue">
              Peels an exact amount off a value, checked against the value's actual size at runtime.
            </OpTourCard>
            <OpTourCard name="split_ratio" tone="blue">
              Divides a value by ratio, exact by construction via largest-remainder allocation.
            </OpTourCard>
            <OpTourCard name="merge" tone="red">
              Combines two same-currency values into one.
            </OpTourCard>
            <Link to="/docs/language/operations-reference" className="card" style={{ display: "flex", flexDirection: "column", justifyContent: "center", alignItems: "flex-start", gap: 6 }}>
              <span style={{ fontWeight: 650, fontSize: 14.5, color: "var(--accent)" }}>Full reference →</span>
              <span style={{ fontSize: 13, color: "var(--text-faint)" }}>Signatures, examples, and every checked precondition.</span>
            </Link>
          </div>
        </div>
      </section>

      <section className="cta-band">
        <div className="wrap">
          <div>
            <h2>See a real bug get caught before it runs.</h2>
            <p>The five-minute walkthrough builds the coffee example, then breaks it on purpose.</p>
          </div>
          <div className="hero-ctas">
            <Link className="btn btn-primary" to="/docs/getting-started">
              Get started
              <svg className="arrow" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2.2} strokeLinecap="round" strokeLinejoin="round">
                <path d="M5 12h14M13 6l6 6-6 6" />
              </svg>
            </Link>
            <Link className="btn btn-secondary" to="/docs/introduction">
              Read the introduction
            </Link>
          </div>
        </div>
      </section>

      <Footer />
    </>
  );
}
