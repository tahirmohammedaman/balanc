import { Link } from "react-router-dom";
import { DocPage } from "../../components/DocPage";
import { C, DocTable, Step, Steps } from "../../components/Prose";
import { Terminal } from "../../components/code/CodeBlock";

export function GettingStarted() {
  return (
    <DocPage
      title="Getting Started"
      lede="Everything below runs from the repository root, building the interpreter straight from source. The interpreter only needs a Rust toolchain; the JVM backend additionally needs a JDK."
    >
      <p>
        Already just want the <C>balanc</C> binary rather than a clone of the repository? See{" "}
        <Link to="/docs/installation">Installation</Link> for the prebuilt-binary and <C>cargo install</C>{" "}
        options — then swap every <C>cargo run --</C> below for a direct <C>balanc</C> call.
      </p>

      <DocTable
        columns={[{ header: "Requirement" }, { header: "Needed for" }]}
        rows={[
          [
            <span className="code-col">Rust toolchain (stable)</span>,
            <>Building/running the interpreter, the CLI, and <C>cargo test</C></>,
          ],
          [
            <span className="code-col">
              JDK 17+ (<C>javac</C>, <C>java</C>, <C>jar</C> on <C>PATH</C>)
            </span>,
            <><C>--emit-jvm</C>, and the JVM differential test suite</>,
          ],
        ]}
      />

      <Steps>
        <Step title="Build and test">
          <p>Standard cargo workflow — no extra setup, no build script.</p>
          <Terminal
            lines={[
              { type: "cmd", text: "cargo build" },
              { type: "cmd", text: "cargo test" },
            ]}
          />
        </Step>

        <Step title="Run the coffee example">
          <p>
            Interprets <C>examples/coffee.bal</C> and prints its trial balance.
          </p>
          <Terminal
            lines={[
              { type: "cmd", text: "cargo run -- examples/coffee.bal" },
              {
                type: "out",
                text: "Trial Balance (ETB)\n  assets:cash                    -45.00\n  expenses:coffee                 45.00\n  Debit total                      0.00\n  Credit total                     0.00",
              },
            ]}
          />
        </Step>

        <Step title="See a diagnostic">
          <p>
            Every money value must be consumed. Comment out the <C>debit</C> line and re-run, and the compiler
            catches the drop before anything executes:
          </p>
          <Terminal
            lines={[
              { type: "cmd", text: "cargo run -- examples/coffee.bal" },
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
            See the full <Link to="/docs/diagnostics">diagnostics reference</Link> for every error code.
          </p>
        </Step>

        <Step title="Machine-readable output">
          <p>
            <C>--json</C> mirrors the same information as structured JSON, on both the success and failure
            paths — useful for editor tooling or CI.
          </p>
          <Terminal
            lines={[
              { type: "cmd", text: "cargo run -- --json examples/coffee.bal" },
              { type: "out", text: '{"ok":true,"report":"Trial Balance (ETB)\\n  assets:cash ..."}' },
            ]}
          />
        </Step>

        <Step title="Compile to a JVM class">
          <p>
            <C>--emit-jvm</C> additionally compiles the typed module to a <C>.class</C> file — the interpreter
            still runs and prints its report in the same invocation. The class name is derived from the file's
            stem, PascalCased.
          </p>
          <Terminal lines={[{ type: "cmd", text: "cargo run -- examples/coffee.bal --emit-jvm Coffee.class" }]} />
          <p>To actually run it, compile the small Java runtime shim alongside it and put both on the classpath:</p>
          <Terminal
            lines={[
              { type: "cmd", text: "javac -d out runtime/balanc/runtime/Ledger.java" },
              { type: "cmd", text: "cp Coffee.class out/" },
              { type: "cmd", text: "java -cp out Coffee" },
              {
                type: "out",
                text: "Trial Balance (ETB)\n  assets:cash                    -45.00\n  expenses:coffee                 45.00\n  Debit total                      0.00\n  Credit total                     0.00",
              },
            ]}
          />
          <p>
            Full detail on what's inside that class file is in{" "}
            <Link to="/docs/architecture/jvm-backend">the JVM backend section</Link>.
          </p>
        </Step>
      </Steps>
    </DocPage>
  );
}
