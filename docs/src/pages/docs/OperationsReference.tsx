import { DocPage } from "../../components/DocPage";
import { C } from "../../components/Prose";
import { CodeBlock } from "../../components/code/CodeBlock";
import { OpCard } from "../../components/OpCard";

export function OperationsReference() {
  return (
    <DocPage title="Operations Reference" lede="Every way to produce, move, or consume a Money value.">
      <OpCard name="credit(account, amount)" tone="green" toneLabel="introduces Money">
        <p>
          Takes money out of <C>account</C> and produces a fresh <C>Money&lt;C&gt;</C> value, where <C>C</C> is
          that account's declared currency. The only way to bring a money value into existence.
        </p>
        <CodeBlock lang="bal" code={`let m = credit(assets:cash, 45.00);`} />
      </OpCard>

      <OpCard name="debit(account, money)" tone="red" toneLabel="consumes Money">
        <p>
          Puts <C>money</C> into <C>account</C>, consuming it. The account's currency must match the value's.
        </p>
        <CodeBlock lang="bal" code={`debit(expenses:coffee, m);`} />
      </OpCard>

      <OpCard name="let name = expr;" tone="blue" toneLabel="binds">
        <p>
          Binds a single <C>MoneyExpr</C> — a <C>credit</C>, a <C>merge</C>, or an existing variable — to a name
          for later consumption.
        </p>
        <CodeBlock lang="bal" code={`let m = credit(assets:cash, 45.00);`} />
      </OpCard>

      <OpCard name="let (p, r) = convert(money, rate);" tone="blue" toneLabel="produces a pair">
        <p>
          Converts <C>money</C> across currencies using a declared <C>rate</C> (
          <C>rate NAME from A to B = VALUE round down;</C>). Produces the converted <C>Money&lt;B&gt;</C> plus
          whatever fraction rounding down left over, as a <C>Residue&lt;B&gt;</C> — never silently discarded.
        </p>
        <CodeBlock
          lang="bal"
          code={`rate usd_etb from USD to ETB = 57.20 round down;\n\nlet (m2, r) = convert(m, usd_etb);`}
        />
      </OpCard>

      <OpCard name="absorb(residue, account)" tone="red" toneLabel="consumes a Residue">
        <p>
          Discharges a conversion residue into an account — conventionally a dedicated rounding account like{" "}
          <C>income:fx_rounding</C>.
        </p>
        <CodeBlock lang="bal" code={`absorb(r, income:fx_rounding);`} />
      </OpCard>

      <OpCard name="let (a, b) = split(money, amount);" tone="blue" toneLabel="produces a pair">
        <p>
          Splits <C>money</C> into an exact <C>amount</C> and whatever remains. <C>amount</C> must not exceed
          the value being split, checked at runtime (<C>E_UNBALANCED</C>).
        </p>
        <CodeBlock lang="bal" code={`let (a, remainder) = split(m, 100.00);`} />
      </OpCard>

      <OpCard name="let (a, b) = split_ratio(money, p, q);" tone="blue" toneLabel="produces a pair">
        <p>
          Splits <C>money</C> by ratio <C>p : q</C> using largest-remainder allocation — exact by construction,
          ties toward <C>a</C>. At least one weight must be nonzero.
        </p>
        <CodeBlock lang="bal" code={`let (b, c) = split_ratio(remainder, 1, 1);`} />
      </OpCard>

      <OpCard name="merge(a, b)" tone="red" toneLabel="consumes two Money">
        <p>
          Combines two same-currency values into one. Unlike <C>convert</C>/<C>split</C>/<C>split_ratio</C>,
          this yields a single value, so it fits anywhere a <C>MoneyExpr</C> is expected — inline in a{" "}
          <C>let</C> or a <C>debit</C> — rather than needing its own <C>let (x, y) = ...</C> form.
        </p>
        <CodeBlock lang="bal" code={`debit(expenses:rent, merge(a, b));`} />
      </OpCard>
    </DocPage>
  );
}
