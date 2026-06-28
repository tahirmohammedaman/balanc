import { DocPage } from "../../components/DocPage";
import { C, DocTable } from "../../components/Prose";
import { Terminal } from "../../components/code/CodeBlock";

export function Diagnostics() {
  return (
    <DocPage
      title="Diagnostics Reference"
      lede="Every diagnostic has a stable code, never reused for a different meaning once released. Diagnostics are sorted by source span, so a file with several independent errors reports all of them in one run instead of stopping at the first."
    >
      <h2>Lex</h2>
      <DocTable
        columns={[{ header: "Code" }, { header: "Meaning" }]}
        rows={[
          [
            <span className="code-col">E_LEX_UNEXPECTED_CHAR</span>,
            "A character does not begin any valid token. Fatal — Slice 0's lexer has no error recovery, so there is never more than one lex diagnostic per run.",
          ],
        ]}
      />

      <h2>Parse</h2>
      <DocTable
        columns={[{ header: "Code" }, { header: "Meaning" }]}
        rows={[
          [
            <span className="code-col">E_PARSE_UNEXPECTED_TOKEN</span>,
            "The parser found a token it cannot use at this point in the grammar.",
          ],
          [
            <span className="code-col">E_PARSE_UNEXPECTED_EOF</span>,
            <>
              End of input reached while a construct (e.g. <C>{`txn { ... }`}</C>) was still open — usually a
              missing <C>{"}"}</C> or <C>;</C>.
            </>,
          ],
          [
            <span className="code-col">E_EXPR_TOO_DEEP</span>,
            <>
              A <C>merge</C>/<C>split</C>/<C>split_ratio</C> chain nests a <C>MoneyExpr</C> more than 256 levels
              deep.
            </>,
          ],
        ]}
      />

      <h2>Resolve (names)</h2>
      <DocTable
        columns={[{ header: "Code" }, { header: "Meaning" }]}
        rows={[
          [
            <span className="code-col">E_UNBOUND_NAME</span>,
            <>A variable refers to a name with no <C>let</C> binding in scope.</>,
          ],
          [
            <span className="code-col">E_UNKNOWN_CURRENCY</span>,
            <>
              An <C>account</C> names a currency with no matching <C>currency</C> declaration.
            </>,
          ],
          [
            <span className="code-col">E_DUPLICATE_CURRENCY</span>,
            <>Two <C>currency</C> declarations use the same name.</>,
          ],
          [
            <span className="code-col">E_DUPLICATE_ACCOUNT</span>,
            <>Two <C>account</C> declarations name the same path.</>,
          ],
          [
            <span className="code-col">E_UNDECLARED_ACCOUNT</span>,
            <>
              A <C>credit</C>/<C>debit</C> names an account path with no <C>account</C> declaration.
            </>,
          ],
          [
            <span className="code-col">E_UNDECLARED_RATE</span>,
            <>
              A <C>convert</C>/<C>absorb</C> names a rate with no matching <C>rate</C> declaration.
            </>,
          ],
          [
            <span className="code-col">E_DUPLICATE_RATE</span>,
            <>Two <C>rate</C> declarations use the same name.</>,
          ],
          [
            <span className="code-col">E_UNKNOWN_ACCOUNT_KIND</span>,
            <>
              An account's <C>kind</C> names something other than one of the five closed values: <C>asset</C>,{" "}
              <C>liability</C>, <C>equity</C>, <C>income</C>, <C>expense</C>.
            </>,
          ],
        ]}
      />

      <h2>Typeck (types &amp; linearity)</h2>
      <DocTable
        columns={[{ header: "Code" }, { header: "Meaning" }]}
        rows={[
          [
            <span className="code-col">E_TOO_MANY_FRACTION_DIGITS</span>,
            "A literal has more fraction digits than its resolved currency's declared scale allows.",
          ],
          [
            <span className="code-col">E_AMOUNT_OUT_OF_RANGE</span>,
            <>
              A literal's magnitude doesn't fit the fixed-point <C>i64</C> representation this language is built
              on.
            </>,
          ],
          [
            <span className="code-col">E_CURRENCY_MISMATCH</span>,
            <>
              A <C>debit</C>'s money and its target account have different currencies (also raised by a
              currency-mismatched <C>convert</C>, <C>absorb</C>, or <C>merge</C>).
            </>,
          ],
          [
            <span className="code-col">E_EXPECTED_MONEY</span>,
            <>
              A <C>debit</C> (or <C>convert</C>'s input) names a binding that resolved to a <C>Residue</C>, not{" "}
              <C>Money</C> — use <C>absorb</C> instead.
            </>,
          ],
          [
            <span className="code-col">E_EXPECTED_RESIDUE</span>,
            <>
              An <C>absorb</C> names a binding that resolved to <C>Money</C>, not a <C>Residue</C> — use{" "}
              <C>debit</C> instead.
            </>,
          ],
          [
            <span className="code-col">E_DROPPED</span>,
            <>
              A bound <C>Money</C> or <C>Residue</C> is never consumed by the end of the transaction body.
            </>,
          ],
          [
            <span className="code-col">E_REUSED</span>,
            "A value is consumed a second time. Carries two spans: the reuse, and where it was first consumed.",
          ],
          [
            <span className="code-col">E_ZERO_RATIO</span>,
            <><C>split_ratio</C>'s two weights are both zero, which doesn't determine an allocation.</>,
          ],
        ]}
      />

      <h2>Eval (runtime side conditions)</h2>
      <DocTable
        columns={[{ header: "Code" }, { header: "Meaning" }]}
        rows={[
          [
            <span className="code-col">E_UNBALANCED</span>,
            <>
              <C>split(e, n)</C> names an <C>n</C> exceeding <C>e</C>'s actual value — the one balance check
              that depends on a runtime magnitude rather than falling out of linearity alone.
            </>,
          ],
        ]}
      />

      <h2>Anatomy of a diagnostic</h2>
      <p>
        Rendered rustc-style: a header with the code and message, a source snippet with a caret at the exact
        span, and a fixed <C>= note:</C> line with general guidance for that code. This is the real output for{" "}
        the dropped-value example in Getting Started:
      </p>
      <Terminal
        lines={[
          {
            type: "out",
            text:
              "error[E_DROPPED]: this money value is never consumed\n" +
              "  --> coffee.bal:6:7\n" +
              "  |\n" +
              "6 |   let m = credit(assets:cash, 45.00);\n" +
              "  |       ^\n" +
              "  = note: every money value bound in a transaction must be consumed exactly\n" +
              "    once — consume it with 'debit', 'absorb', 'split', 'split_ratio', or 'merge'",
          },
        ]}
      />
      <p>
        <C>--json</C> renders the same information as{" "}
        <C>{`{"ok":false,"diagnostics":[{"code","message","file","line","col","span","secondary"}]}`}</C>{" "}
        instead — one object per diagnostic, each secondary span carrying its own <C>label</C>, <C>line</C>, and{" "}
        <C>col</C>.
      </p>
    </DocPage>
  );
}
