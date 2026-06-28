import { DocPage } from "../../components/DocPage";
import { C, DocTable } from "../../components/Prose";
import { CodeBlock } from "../../components/code/CodeBlock";

export function CoreConcepts() {
  return (
    <DocPage
      title="Core Concepts"
      lede="Four building blocks: currencies, accounts, transactions, and money itself. Everything else in the language is an operation over these."
    >
      <h2>Currencies</h2>
      <p>
        A <C>currency</C> declaration fixes a name and a <em>scale</em> — how many fractional digits its minor
        unit has. Scale is a per-currency property, not a language constant, so ETB and a hypothetical
        zero-decimal currency like JPY can coexist in the same program.
      </p>
      <CodeBlock lang="bal" code={`currency ETB { scale = 2 }`} />

      <h2>Accounts</h2>
      <p>
        An <C>account</C> declaration binds a colon-separated path to a currency, a <em>kind</em>, and a{" "}
        <em>normal balance</em> side.
      </p>
      <CodeBlock lang="bal" code={`account assets:cash { currency = ETB, kind = asset, normal = debit }`} />
      <DocTable
        columns={[{ header: "Field" }, { header: "Meaning" }]}
        rows={[
          [<span className="code-col">currency</span>, "Which declared currency this account holds."],
          [
            <span className="code-col">kind</span>,
            <>
              One of the five closed accounting categories: <C>asset</C>, <C>liability</C>, <C>equity</C>,{" "}
              <C>income</C>, <C>expense</C>. Used only to group and order the trial balance report.
            </>,
          ],
          [
            <span className="code-col">normal</span>,
            <>
              <C>debit</C> or <C>credit</C> — which side this account displays a positive balance on. A
              <em> display-time</em> convention only: the underlying ledger arithmetic always treats debit as
              "add" and credit as "subtract," uniformly, regardless of an account's kind or normal side.
            </>,
          ],
        ]}
      />

      <h2>Transactions</h2>
      <p>
        A <C>{`txn "name" { ... }`}</C> block is <strong>closed</strong>: it starts with nothing bound, and
        every value created inside it must be consumed inside it. Nothing — no money, no partially-used binding
        — crosses a transaction boundary.
      </p>

      <h2>Money is linear</h2>
      <p>
        <C>credit</C> is the only way to introduce a <C>Money</C> value; a bare numeric literal is never money
        on its own. Once introduced, that value must be consumed exactly once, by exactly one of <C>debit</C>,{" "}
        <C>absorb</C>, <C>split</C>, <C>split_ratio</C>, or as an argument to <C>merge</C>. The full mechanics
        are formalized in the type system reference next.
      </p>
    </DocPage>
  );
}
