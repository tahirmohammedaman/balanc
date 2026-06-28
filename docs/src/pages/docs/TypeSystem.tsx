import { DocPage } from "../../components/DocPage";
import { C, DocTable, Rule } from "../../components/Prose";

export function TypeSystem() {
  return (
    <DocPage
      title="The Type System"
      lede="balanc's type checker tracks a linear context Γ — the set of not-yet-consumed bindings in the current transaction — and proves it ends empty. The rules below are the actual inference rules from the source, written before the code that implements them."
    >
      <h2>Types</h2>
      <DocTable
        columns={[{ header: "Type" }, { header: "Meaning" }]}
        rows={[
          [
            <span className="code-col">Money&lt;C&gt;</span>,
            <>
              An ordinary linear money value in currency <C>C</C>. Currency is a phantom type parameter — it
              exists purely to keep the checker from mixing currencies, and costs nothing at runtime (money is
              erased to a plain integer by the time either backend runs it).
            </>,
          ],
          [
            <span className="code-col">Residue&lt;C&gt;</span>,
            <>
              What <C>convert</C> hands back for the fraction lost to rounding. Carries real value, but can only
              be discharged by <C>absorb</C>, never <C>debit</C> — using the wrong one is <C>E_EXPECTED_MONEY</C>{" "}
              / <C>E_EXPECTED_RESIDUE</C>.
            </>,
          ],
          [
            <span className="code-col">Amount</span>,
            <>A literal numeric argument (e.g. <C>credit</C>'s second argument) — not itself a linear value.</>,
          ],
          [
            <span className="code-col">Effect</span>,
            <>
              What <C>debit</C>/<C>absorb</C> produce — a side effect on the ledger, not a value a program can
              bind or pass around.
            </>,
          ],
          [<span className="code-col">Txn</span>, "A whole, closed transaction body."],
        ]}
      />

      <h2>Linearity: Γ and Δ</h2>
      <p>
        Γ is the linear context: live bindings in scope, each tagged with the currency it was produced at and
        whether it's <C>Money</C> or a <C>Residue</C>. Consuming a binding (e.g. as <C>debit</C>'s argument)
        removes it from Γ. At the end of a transaction body, whatever remains — Δ — must be empty:
      </p>
      <Rule
        premises={"Γ ⊢ body ⇒ Δ    Δ = ∅"}
        name="T-Txn"
        conclusion={"⊢ txn { body } : Txn"}
        side={
          <>
            Every value bound anywhere in the transaction was consumed by the time it closes. A binding still
            live at the end is reported as <C>E_DROPPED</C>; consuming the same binding twice is <C>E_REUSED</C>,
            and carries both use-sites in the diagnostic.
          </>
        }
      />

      <h2>The rules</h2>

      <Rule
        premises={"acct : Account<C>    Γ ⊢ n : Amount"}
        name="T-Credit"
        conclusion={"Γ ⊢ credit(acct, n) : Money<C>"}
        side="Introduces a fresh linear value. credit is the only place a value's currency is first fixed — every other rule forwards a currency it already has."
      />

      <Rule
        premises={"Γ ⊢ e : Money<C>    acct : Account<C>"}
        name="T-Debit"
        conclusion={"Γ ⊢ debit(acct, e) : Effect    (consumes e)"}
        side={
          <>
            The account's declared currency must match <C>e</C>'s — a mismatch is <C>E_CURRENCY_MISMATCH</C>,
            with a secondary span pointing at where <C>e</C>'s currency was established.
          </>
        }
      />

      <Rule
        premises={"Γ ⊢ e : Money<A>    rate : Rate<A, B>"}
        name="T-Convert"
        conclusion={"Γ ⊢ let (p, r) = convert(e, rate) : Money<B> ⊗ Residue<B>"}
        side={
          <>
            Consumes <C>e</C>; binds <C>p : Money&lt;B&gt;</C> and <C>r : Residue&lt;B&gt;</C>. The residue is
            real money that must itself be discharged — conversion can never make a fraction of a unit silently
            disappear.
          </>
        }
      />

      <Rule
        premises={"Γ ⊢ x : Residue<C>    acct : Account<C>"}
        name="T-Absorb"
        conclusion={"Γ ⊢ absorb(x, acct) : Effect    (consumes x)"}
        side={
          <>
            The only way to discharge a <C>Residue</C> — typically into a dedicated rounding account, e.g.{" "}
            <C>income:fx_rounding</C>.
          </>
        }
      />

      <Rule
        premises={"Γ ⊢ e : Money<C>    n : Amount"}
        name="T-Split"
        conclusion={"Γ ⊢ let (a, b) = split(e, n) : Money<C> ⊗ Money<C>"}
        side={
          <>
            Consumes <C>e</C>; binds <C>a = n</C> and <C>b = amount(e) - n</C>. Side condition{" "}
            <C>0 ≤ n ≤ amount(e)</C> — unlike every rule above, <C>amount(e)</C> is a runtime quantity the type
            checker never sees, so this one condition is checked in the evaluator instead, as{" "}
            <C>E_UNBALANCED</C>. It's the one place "the debits and credits don't agree" is a genuine computed
            check rather than something linearity rules out structurally.
          </>
        }
      />

      <Rule
        premises={"Γ ⊢ e : Money<C>    p, q : Nat    p + q > 0"}
        name="T-SplitRatio"
        conclusion={"Γ ⊢ let (a, b) = split_ratio(e, p, q) : Money<C> ⊗ Money<C>"}
        side={
          <>
            Binds a largest-remainder allocation of <C>amount(e)</C> in ratio <C>p : q</C>, ties broken toward{" "}
            <C>a</C> — exact by construction (<C>a + b</C> always equals <C>amount(e)</C>), so unlike{" "}
            <C>split</C> there's no runtime bound check, only the literal-checkable <C>p + q &gt; 0</C> (
            <C>E_ZERO_RATIO</C> otherwise).
          </>
        }
      />

      <Rule
        premises={"Γ ⊢ a : Money<C>    Γ ⊢ b : Money<C>"}
        name="T-Merge"
        conclusion={"Γ ⊢ merge(a, b) : Money<C>"}
        side={
          <>
            Consumes <C>a</C>, then <C>b</C>, from the same Γ — which is also why <C>merge(m, m)</C> is
            rejected: the second reference to <C>m</C> is just an ordinary reuse of an already-consumed binding,
            no extra machinery needed.
          </>
        }
      />

      <h2>How the balance premise evolved</h2>
      <p>
        Through the language's early slices, "debits and credits agree" was never a separate check — it fell
        out of linearity for free. The grammar had exactly one way to create a <C>Money</C> value (<C>credit</C>)
        and exactly one way to destroy one (<C>debit</C>), with no arithmetic in between, so <C>Δ = ∅</C> alone
        forced every credited amount to reach exactly one same-currency debit. <C>convert</C> was the first
        thing to break that argument — a credited value can now leave its currency's books entirely — which is
        correct once conversion exists, not a regression: <C>Δ = ∅</C> still guarantees nothing is dropped, it
        just no longer implies each currency nets to zero on its own. <C>split</C> is where a real, computed
        balance check (<C>E_UNBALANCED</C>) becomes necessary for the first time, because it's the first
        operation whose output magnitude depends on a runtime value the type checker can't see.
      </p>
    </DocPage>
  );
}
