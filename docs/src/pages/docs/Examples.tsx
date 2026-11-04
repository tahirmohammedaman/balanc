import { DocPage } from "../../components/DocPage";
import { ProgramBlock } from "../../components/code/CodeBlock";

export function Examples() {
  return (
    <DocPage
      title="Examples Gallery"
      lede="Four programs, each exercised by the differential test suite against both backends, shown with the interpreter's real output."
    >
      <h2>coffee — the minimal transaction</h2>
      <p>One currency, two accounts, one transaction: credit cash, debit an expense.</p>
      <ProgramBlock path="examples/coffee.bal" />

      <h2>multi_currency — independent currency sections</h2>
      <p>
        Two currencies, each with their own accounts and transactions. The trial balance renders one section
        per currency that had at least one posting — a currency untouched in a given run gets no section at
        all.
      </p>
      <ProgramBlock path="tests/cases/multi_currency.bal" />

      <h2>convert_and_absorb — crossing currencies with an accounted-for residue</h2>
      <p>
        Converts USD to ETB at a declared rate, rounding down, and explicitly absorbs the rounding residue into
        an income account rather than letting it disappear.
      </p>
      <ProgramBlock path="tests/cases/convert_and_absorb.bal" />

      <h2>split_and_merge — dividing and recombining a value</h2>
      <p>
        Splits an exact amount off, then splits the remainder by ratio, then merges two of the resulting pieces
        back into one before debiting it.
      </p>
      <ProgramBlock path="tests/cases/split_and_merge.bal" />
    </DocPage>
  );
}
