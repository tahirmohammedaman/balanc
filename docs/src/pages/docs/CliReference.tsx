import { DocPage } from "../../components/DocPage";
import { C, DocTable } from "../../components/Prose";
import { CodeBlock, Terminal } from "../../components/code/CodeBlock";

export function CliReference() {
  return (
    <DocPage title="CLI Reference" lede="One binary, balanc, hand-parsed with no subcommands.">
      <CodeBlock lang="text" code={`usage: balanc [--json] [--emit-jvm <out.class>] <file.bal>`} />

      <DocTable
        columns={[{ header: "Flag" }, { header: "Effect" }]}
        rows={[
          [
            <span className="code-col">--json</span>,
            <>
              Emits <C>{`{"ok":true,"report":...}`}</C> on success, or{" "}
              <C>{`{"ok":false,"diagnostics":[...]}`}</C> on failure — to stdout either way — instead of the
              plain-text report/stderr diagnostics.
            </>,
          ],
          [
            <span className="code-col">--emit-jvm &lt;out.class&gt;</span>,
            "After a successful typecheck, additionally compiles the module to JVM bytecode and writes it to the given path. Additive, not a mode switch — the interpreter still runs and prints its report in the same invocation.",
          ],
          [<span className="code-col">&lt;file.bal&gt;</span>, "The source file to run (required, positional)."],
        ]}
      />

      <p>
        On a typecheck or evaluation failure, diagnostics print to stderr (or stdout under <C>--json</C>) and
        the process exits with a non-zero status. No class file is written if typechecking fails — there's no
        typed module to compile.
      </p>

      <Terminal
        lines={[
          { type: "cmd", text: "cargo run -- examples/coffee.bal" },
          { type: "cmd", text: "cargo run -- examples/coffee.bal --emit-jvm Coffee.class" },
          { type: "cmd", text: "cargo run -- --json examples/coffee.bal" },
        ]}
      />
    </DocPage>
  );
}
