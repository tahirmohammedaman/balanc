import { DocPage } from "../../components/DocPage";
import { C, DocTable } from "../../components/Prose";
import { PipelineDiagram } from "../../components/PipelineDiagram";

export function CompilerArchitecture() {
  return (
    <DocPage
      title="Compiler Architecture"
      lede="Source text passes through six stages. Each stage owns its own output type and never reaches back into a previous one — typeck in particular never imports eval, so nothing type-related can leak downstream of it."
    >
      <DocTable
        columns={[{ header: "Stage" }, { header: "Input → Output" }, { header: "Responsibility" }]}
        rows={[
          [
            <span className="code-col">lex</span>,
            <span className="code-col">&amp;str → Vec&lt;Token&gt;</span>,
            "Hand-written, no generator. Tokens carry source spans; nothing downstream touches source text directly except diagnostic rendering. No error recovery — the first fatal error stops lexing.",
          ],
          [
            <span className="code-col">parse</span>,
            <span className="code-col">Vec&lt;Token&gt; → ast::Module</span>,
            "Hand-written recursive descent, one or two tokens of lookahead, never backtracks. Malformed declarations/statements are discarded and parsing resynchronizes at the next keyword or ;/}, so one run can report several independent syntax errors.",
          ],
          [
            <span className="code-col">resolve</span>,
            <span className="code-col">ast::Module → ResolvedModule</span>,
            <>
              Turns every name into an id — <C>SymbolId</C>, <C>CurrencyId</C>, <C>AccountId</C>, <C>RateId</C>{" "}
              — and validates the closed <C>AccountKind</C> set.
            </>,
          ],
          [
            <span className="code-col">typeck</span>,
            <span className="code-col">ResolvedModule → TModule</span>,
            "Runs the T-* inference rules over a linear context Γ per transaction, proving Δ = ∅. Produces the single typed IR both backends consume.",
          ],
          [
            <span className="code-col">eval</span>,
            <span className="code-col">TModule → LedgerState</span>,
            <>
              Tree-walking interpreter. <C>LedgerState</C> is a <C>BTreeMap&lt;String, Amount&gt;</C> from
              account path to running signed balance — debits (and absorbs) add, credits subtract, uniformly.
            </>,
          ],
          [
            <span className="code-col">render</span>,
            <span className="code-col">LedgerState → String</span>,
            "The only stage besides the CLI's main allowed to produce human output. Turns raw balances into the trial balance report.",
          ],
        ]}
      />

      <PipelineDiagram />

      <h2>Why fixed-point, never floating point</h2>
      <p>
        Amounts are stored as a signed integer count of the currency's minor unit (an <C>i64</C> of cents), with
        scale tracked per-currency rather than baked into the number. Ledger arithmetic has to be exact and
        explainable — <C>f64</C> can't represent 0.10 exactly, and an error that looks like a rounding drift in
        a bookkeeping system is indistinguishable from a real bug. Currency conversion computes its exact
        rational result via <C>i128</C> intermediate arithmetic, so a <C>convert</C>'s split into the primary
        amount and its residue is always exact, never lossy beyond the one deliberate truncation the operation
        performs.
      </p>

      <h2>Diagnostics infrastructure</h2>
      <p>
        Every diagnostic is a <C>{`Diagnostic { code, message, span, secondary }`}</C>, rendered either as
        rustc-style text or as JSON via <C>--json</C>. Diagnostics are always sorted by source span before being
        printed, so multi-error output is deterministic regardless of the order the compiler happened to
        discover the errors in. Every diagnostic is required to carry a real, non-synthetic span — enforced by
        a dedicated test across the entire example corpus.
      </p>
    </DocPage>
  );
}
