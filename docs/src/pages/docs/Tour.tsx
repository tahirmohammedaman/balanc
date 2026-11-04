import { Link } from "react-router-dom";
import { DocPage } from "../../components/DocPage";
import { C, Callout } from "../../components/Prose";
import { ProgramBlock } from "../../components/code/CodeBlock";

export function Tour() {
  return (
    <DocPage
      title="Language Tour"
      lede="Ten short programs, from one coffee to a currency conversion with a rounding residue. Each one is a file in the repository, and the output under it is what the compiler printed for that file."
    >
      <Callout title="How to follow along">
        <p>
          Every block names its file. With balanc installed, run <C>balanc &lt;file&gt;</C> from a checkout of the
          repository and you should see the same output, byte for byte. A program that type-checks is interpreted and
          prints a trial balance per currency; one that doesn’t prints diagnostics and exits with status 1.
        </p>
      </Callout>

      <h2 id="a-transaction">1. A transaction moves money</h2>
      <p>
        A program declares its currencies and accounts, then posts transactions. <C>credit(acct, amount)</C> takes
        money out of an account and gives you a <C>Money&lt;ETB&gt;</C> value; <C>debit(acct, value)</C> puts that
        value into another account and uses it up.
      </p>
      <ProgramBlock path="examples/coffee.bal" />
      <p>
        <C>scale = 2</C> says ETB amounts have two decimal places. <C>kind</C> and <C>normal</C> are the account’s
        accounting classification and normal balance side.
      </p>

      <h2 id="forget-it">2. Money can’t be forgotten</h2>
      <p>
        <C>m</C> is a linear value: it has to be consumed before the transaction closes. Delete the <C>debit</C> and
        the 45.00 leaves <C>assets:cash</C> for nowhere, which the type checker reports at the binding:
      </p>
      <ProgramBlock path="examples/coffee_forgotten.bal" />

      <h2 id="spend-it-twice">3. …or spent twice</h2>
      <p>
        The opposite mistake. A consumed value is gone from the context, so a second use is an error, and the
        diagnostic carries both sites:
      </p>
      <ProgramBlock path="examples/coffee_twice.bal" />

      <h2 id="split">4. Split a value to pay two accounts</h2>
      <p>
        <C>split(m, n)</C> consumes <C>m</C> and binds two new values: exactly <C>n</C>, and whatever is left. Each
        is then consumed once, so nothing is created or lost.
      </p>
      <ProgramBlock path="examples/coffee_split.bal" />

      <h2 id="ratio-and-merge">5. Ratios and merging</h2>
      <p>
        <C>split_ratio(m, p, q)</C> divides by weight, using a largest-remainder allocation so the two parts always
        sum to the input. <C>merge(a, b)</C> joins two values of the same currency into one.
      </p>
      <ProgramBlock path="tests/cases/split_and_merge.bal" />

      <h2 id="unbalanced">6. You can’t split off more than you have</h2>
      <p>
        The type checker doesn’t know amounts, only currencies and linearity. Whether <C>n</C> fits inside{" "}
        <C>m</C> is checked by the evaluator, which is the one place a balance check is computed rather than ruled out
        by the types:
      </p>
      <ProgramBlock path="tests/cases/split_unbalanced.bal" />

      <h2 id="currencies">7. Currencies are types</h2>
      <p>
        Money is <C>Money&lt;C&gt;</C> for a declared currency <C>C</C>, and an account accepts only its own
        currency. The diagnostic also shows where the value’s currency was fixed:
      </p>
      <ProgramBlock path="tests/cases/currency_mismatch.bal" />

      <h2 id="convert">8. Crossing currencies takes a rate</h2>
      <p>
        A <C>rate</C> declaration fixes a conversion and its rounding direction. <C>convert(m, rate)</C> consumes{" "}
        <C>m</C> and binds two values: the converted <C>Money&lt;ETB&gt;</C> and a <C>Residue&lt;ETB&gt;</C> for the
        fraction lost to rounding. A residue is discharged with <C>absorb</C>, typically into a rounding account.
      </p>
      <ProgramBlock path="tests/cases/convert_and_absorb.bal" />

      <h2 id="residue">9. The residue is money too</h2>
      <p>
        Leave the residue unabsorbed and it is dropped, exactly like ordinary money. Using <C>debit</C> on it instead
        of <C>absorb</C> is its own error, <Link to="/docs/errors#E_EXPECTED_MONEY">E_EXPECTED_MONEY</Link>.
      </p>
      <ProgramBlock path="tests/cases/dropped_residue.bal" />

      <h2 id="many-currencies">10. One program, several currencies</h2>
      <p>Transactions in different currencies coexist; the report prints one trial balance per currency.</p>
      <ProgramBlock path="tests/cases/multi_currency.bal" />

      <h2 id="next">Where next</h2>
      <ul>
        <li>
          <Link to="/docs/errors">Error catalogue</Link>: all 21 diagnostic codes, each with a program that triggers
          it.
        </li>
        <li>
          <Link to="/docs/language/type-system">The type system</Link>: the inference rules behind everything above.
        </li>
        <li>
          <Link to="/docs/language/operations-reference">Operations reference</Link>: every money operation in
          detail.
        </li>
        <li>
          <Link to="/docs/architecture/jvm-backend">JVM backend</Link>: compiling the same programs to{" "}
          <C>.class</C> files.
        </li>
      </ul>
    </DocPage>
  );
}
