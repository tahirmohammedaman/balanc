import { DocPage } from "../../components/DocPage";
import { C } from "../../components/Prose";
import { GlossaryEntry } from "../../components/Glossary";

export function Glossary() {
  return (
    <DocPage title="Glossary" lede="The same concepts, defined once, in the vocabulary each audience already has.">
      <dl className="glossary">
        <GlossaryEntry
          term="Linear type"
          dev={
            <>
              A type whose values must be used exactly once — not zero times, not twice. balanc's type checker
              enforces this for <C>Money</C> and <C>Residue</C> the same way a borrow checker tracks ownership.
            </>
          }
          fin="Every unit of money in a transaction is accounted for exactly once — it can't be forgotten (dropped) or counted twice (duplicated)."
        />

        <GlossaryEntry
          term="Residue<C>"
          dev={
            <>
              The second half of what <C>convert</C> returns — a linear value distinct from <C>Money</C>,
              discharged only by <C>absorb</C>.
            </>
          }
          fin="The fraction of a unit lost to rounding when converting currencies — real value that still has to land somewhere, typically an FX-rounding income account."
        />

        <GlossaryEntry
          term="Normal balance"
          dev={
            <>
              A display-only flag (<C>debit</C> or <C>credit</C>) on an account, used solely by <C>render</C> to
              decide whether to flip a raw balance's sign.
            </>
          }
          fin="Which side of the ledger an account type is expected to carry a positive balance on — assets and expenses are normally debit; liabilities, equity, and income are normally credit."
        />

        <GlossaryEntry
          term="Trial balance"
          dev={
            <>
              The output of <C>render::render_trial_balance</C> — one section per currency, accounts grouped by
              kind, each displayed at its normal-balance sign.
            </>
          }
          fin="The standard bookkeeping report listing every account's balance, with debit and credit columns that should reconcile — the accounting equation, made visible."
        />

        <GlossaryEntry
          term="Phantom type parameter"
          dev="A type parameter (here, currency) that exists purely at compile time to keep the checker honest, with zero runtime representation — money is erased to a plain integer by the time either backend actually runs it."
          fin="The guarantee that ETB and USD can never be silently added together — the compiler treats them as genuinely different kinds of value, the same way you would on paper."
        />

        <GlossaryEntry
          term="Conservation"
          dev={
            <>
              The property that total-preserving operations (<C>split</C>, <C>split_ratio</C>, <C>merge</C>)
              never change the sum of value they operate over — proven by property tests, not just asserted.
            </>
          }
          fin="Money in equals money out — splitting or recombining a balance never creates or destroys value."
        />

        <GlossaryEntry
          term="Span"
          dev="A {lo, hi} byte-offset range every token and diagnostic carries, resolved back to a line/column only when rendering — never used for logic upstream."
          fin="The exact place in the source file an error points to — the same idea as a line reference on an audit exception."
        />

        <GlossaryEntry
          term="Ledger"
          dev={
            <>
              At runtime, a <C>BTreeMap&lt;String, Amount&gt;</C> (interpreter) or a{" "}
              <C>balanc.runtime.Ledger</C> instance (JVM backend) mapping account path to running signed
              balance.
            </>
          }
          fin="The running record of every account's balance as transactions post to it — what the trial balance is a snapshot of."
        />
      </dl>
    </DocPage>
  );
}
