import { DocPage } from "../../components/DocPage";
import { CodeBlock } from "../../components/code/CodeBlock";

const COFFEE = `currency ETB { scale = 2 }

account assets:cash { currency = ETB, kind = asset, normal = debit }
account expenses:coffee { currency = ETB, kind = expense, normal = debit }

txn "coffee" {
  let m = credit(assets:cash, 45.00);
  debit(expenses:coffee, m);
}`;

const MULTI_CURRENCY = `currency ETB { scale = 2 }
currency USD { scale = 2 }

account assets:etb_cash { currency = ETB, kind = asset, normal = debit }
account expenses:coffee { currency = ETB, kind = expense, normal = debit }
account assets:usd_cash { currency = USD, kind = asset, normal = debit }
account expenses:software { currency = USD, kind = expense, normal = debit }

txn "coffee" {
  debit(expenses:coffee, credit(assets:etb_cash, 45.00));
}

txn "saas" {
  debit(expenses:software, credit(assets:usd_cash, 10.00));
}`;

const CONVERT_AND_ABSORB = `currency USD { scale = 2 }
currency ETB { scale = 2 }

account assets:usd_cash { currency = USD, kind = asset, normal = debit }
account assets:etb_cash { currency = ETB, kind = asset, normal = debit }
account income:fx_rounding { currency = ETB, kind = income, normal = credit }

rate usd_etb from USD to ETB = 57.20 round down;

txn "fx" {
  let m = credit(assets:usd_cash, 100.01);
  let (m2, r) = convert(m, usd_etb);
  debit(assets:etb_cash, m2);
  absorb(r, income:fx_rounding);
}`;

const SPLIT_AND_MERGE = `currency ETB { scale = 2 }

account assets:cash { currency = ETB, kind = asset, normal = debit }
account expenses:rent { currency = ETB, kind = expense, normal = debit }
account expenses:utilities { currency = ETB, kind = expense, normal = debit }

txn "split rent three ways" {
  let m = credit(assets:cash, 300.00);
  let (a, remainder) = split(m, 100.00);
  let (b, c) = split_ratio(remainder, 1, 1);
  debit(expenses:rent, merge(a, b));
  debit(expenses:utilities, c);
}`;

export function Examples() {
  return (
    <DocPage
      title="Examples Gallery"
      lede="Four programs, each exercised by the differential test suite against both backends."
    >
      <h2>coffee — the minimal transaction</h2>
      <p>One currency, two accounts, one transaction: credit cash, debit an expense.</p>
      <CodeBlock lang="bal" code={COFFEE} />

      <h2>multi_currency — independent currency sections</h2>
      <p>
        Two currencies, each with their own accounts and transactions. The trial balance renders one section
        per currency that had at least one posting — a currency untouched in a given run gets no section at
        all.
      </p>
      <CodeBlock lang="bal" code={MULTI_CURRENCY} />

      <h2>convert_and_absorb — crossing currencies with an accounted-for residue</h2>
      <p>
        Converts USD to ETB at a declared rate, rounding down, and explicitly absorbs the rounding residue into
        an income account rather than letting it disappear.
      </p>
      <CodeBlock lang="bal" code={CONVERT_AND_ABSORB} />

      <h2>split_and_merge — dividing and recombining a value</h2>
      <p>
        Splits an exact amount off, then splits the remainder by ratio, then merges two of the resulting pieces
        back into one before debiting it.
      </p>
      <CodeBlock lang="bal" code={SPLIT_AND_MERGE} />
    </DocPage>
  );
}
