import { DocPage } from "../../components/DocPage";
import { Badge, C, Callout } from "../../components/Prose";

export function Introduction() {
  return (
    <DocPage title="Introduction">
      <p>
        balanc is a statically typed language for double-entry bookkeeping, in which ledger invariants —
        conservation, balance, no value dropped or duplicated — are enforced by the type system instead of
        checked at runtime.
      </p>
      <p>
        <Badge tone="blue">interpreter</Badge> <Badge tone="green">JVM backend</Badge> <Badge tone="amber">MIT licensed</Badge>{" "}
        <Badge tone="blue">zero dependencies</Badge>
      </p>

      <p>Money in balanc is a <strong>linear</strong> resource: every value must be consumed exactly once.</p>
      <ul>
        <li>Dropping a value is a compile error — money would vanish.</li>
        <li>Using a value twice is a compile error — money would be duplicated.</li>
        <li>A transaction whose debits and credits do not agree does not compile.</li>
      </ul>
      <p>
        Money is parameterized by currency, so <C>Money&lt;ETB&gt;</C> and <C>Money&lt;USD&gt;</C> are distinct
        types that cannot be combined; crossing between them requires an explicit <C>rate</C> value at the
        conversion site. Values are only produced and consumed through total-preserving operations —{" "}
        <C>credit</C>, <C>debit</C>, <C>split</C>, <C>merge</C> — and a bare number can never become money
        inside a transaction body.
      </p>

      <h2>Status</h2>
      <p>
        A tree-walking interpreter (<C>balanc::run</C>) and a JVM bytecode backend (<C>balanc::backend::jvm</C>)
        both consume the same typed IR (<C>typeck::TModule</C>). The JVM backend hand-writes <C>.class</C> bytes
        directly in Rust — no ASM, no bytecode-manipulation library — and packages the result as an ordinary,
        framework-free <C>.jar</C>. Both backends are exercised by a differential test suite that asserts they
        produce byte-identical trial balances for the same program.
      </p>

      <Callout title="Design stance">
        <p>
          balanc has zero runtime dependencies and zero build dependencies — the lexer, parser, JVM class-file
          writer, and JSON diagnostic output are all hand-written. Where the project reaches for something
          off-the-shelf, it's the JDK's own <C>javac</C>/<C>java</C>/<C>jar</C>/<C>javap</C>, used only as the
          JVM backend's runtime and test oracle, never as a build-time dependency of the Rust side.
        </p>
      </Callout>
    </DocPage>
  );
}
