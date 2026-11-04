import { Link } from "react-router-dom";
import { DocPage } from "../../components/DocPage";
import { C, DocTable, Step, Steps } from "../../components/Prose";
import { RealOutput, Terminal } from "../../components/code/CodeBlock";
import { program, terminalText } from "../../data/programs";

// The real `--json` success output for coffee.bal is the report wrapped in
// {"ok":true,"report":...}; reconstructed from the generated plain output.
const JSON_OK = JSON.stringify({ ok: true, report: program("examples/coffee.bal").stdout });

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
              { type: "out", text: terminalText(program("examples/coffee.bal")) },
            ]}
          />
        </Step>

        <Step title="See a diagnostic">
          <p>
            Every money value must be consumed. Delete the <C>debit</C> line (the result is checked in as{" "}
            <C>examples/coffee_forgotten.bal</C>) and the compiler catches the drop before anything executes:
          </p>
          <RealOutput path="examples/coffee_forgotten.bal" prefix="cargo run --" />
          <p>
            The <Link to="/docs/errors">error catalogue</Link> lists every code, each with a program that triggers it.
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
              { type: "out", text: JSON_OK },
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
              { type: "out", text: terminalText(program("examples/coffee.bal")) },
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
