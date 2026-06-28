import { DocPage } from "../../components/DocPage";
import { C, Callout, DocTable } from "../../components/Prose";
import { CodeBlock, Terminal } from "../../components/code/CodeBlock";

const LEDGER_API = `public final class Ledger {
    public Ledger(int accountCount)

    // Posts \`delta\` to \`account\`'s running balance (debits positive, credits negative).
    public void post(int account, long delta)

    // T-SplitRatio's largest-remainder allocation, ties toward \`a\`.
    public static long[] splitRatio(long total, long aWeight, long bWeight)

    // T-Split's runtime bound check; throws IllegalStateException if amount > total.
    public static long checkedSplitRemainder(long total, long amount)

    // Builds the full trial-balance report string, mirroring render::render_trial_balance.
    public String report(
        String[] paths, int[] currencyIndex, int[] kindOrder,
        boolean[] isDebitNormal, String[] currencyNames, int[] currencyScales)
}`;

export function JvmBackend() {
  return (
    <DocPage
      title="JVM Backend"
      lede="Compiles the same typeck::TModule the interpreter runs into a single hand-written .class file — no ASM, no bytecode-manipulation library. The only Java source in the whole project is the small runtime shim below; nothing is generated as Java source and fed through javac per module."
    >
      <Callout title="Why hand-write the class file">
        <p>
          The bytecode surface this language needs is small and fully bounded — no generics, no exceptions, no{" "}
          <C>invokedynamic</C>. Writing the JVMS §4.4 constant pool and method table by hand in Rust keeps the
          whole toolchain to "a Rust compiler plus a JDK," with no second build-time dependency to version or
          vendor.
        </p>
      </Callout>

      <h2>Module layout</h2>
      <DocTable
        columns={[{ header: "Module" }, { header: "Responsibility" }]}
        rows={[
          [
            <span className="code-col">backend::jvm::pool</span>,
            "The constant pool (JVMS §4.4) — interns UTF-8, class, name-and-type, field/method-ref, string, integer, and long entries.",
          ],
          [
            <span className="code-col">backend::jvm::code</span>,
            <>
              The <C>Code</C> attribute builder — raw opcode emission with automatic <C>max_stack</C>/
              <C>max_locals</C> tracking. No jump/label/patch machinery at all.
            </>,
          ],
          [
            <span className="code-col">backend::jvm::class</span>,
            <>
              Assembles a complete <C>.class</C> file — magic number, constant pool, access flags, fields,
              methods — targeting class file major version 61 (Java 17), for the widest compatible floor.
            </>,
          ],
          [
            <span className="code-col">backend::jvm::codegen</span>,
            <>
              Walks a <C>TModule</C> and drives the three modules above to emit one class's bytes.
            </>,
          ],
        ]}
      />

      <h2>Why no branches, ever</h2>
      <p>
        Every method this backend emits is straight-line bytecode — no jump instruction, and therefore no{" "}
        <C>StackMapTable</C> attribute to generate (a requirement for any class file with a branch, targeting
        anything past Java 6). That's possible because every place this language's own semantics has a genuine{" "}
        <em>data-dependent</em> choice — <C>split_ratio</C>'s largest-remainder tie-break, <C>split</C>'s
        runtime bound check, and even the trial balance report's per-account grouping logic — is pushed into a
        single method call into the Java runtime shim below, which carries the branch instead, as ordinary{" "}
        <C>javac</C>-compiled Java. Money itself is fully erased to a <C>long</C> of minor units by this point:
        linearity was already discharged by <C>typeck</C>, so a <C>let</C>/<C>convert</C>/<C>split</C>/
        <C>split_ratio</C> binding becomes one <C>long</C> local-variable slot.
      </p>

      <h2>Anatomy of a generated class</h2>
      <p>
        One class per <C>.bal</C> module, named from the source file's stem (PascalCased, e.g.{" "}
        <C>split_and_merge.bal</C> → <C>SplitAndMerge</C>):
      </p>
      <DocTable
        columns={[{ header: "Member" }, { header: "What it does" }]}
        rows={[
          [
            <span className="code-col">ledger</span>,
            <>
              A <C>private final balanc.runtime.Ledger</C> instance field, indexed by account, so no name lookup
              is needed at runtime.
            </>,
          ],
          [
            <span className="code-col">
              PATHS, CURRENCY_INDEX, KIND_ORDER, IS_DEBIT_NORMAL, CURRENCY_NAMES, CURRENCY_SCALES
            </span>,
            <>
              Static metadata arrays populated in <C>&lt;clinit&gt;</C>, compile-time-known from the module's
              declarations — the report only needs balances at runtime.
            </>,
          ],
          [
            <span className="code-col">&lt;init&gt;()</span>,
            <>Constructs a fresh <C>Ledger</C>.</>,
          ],
          [
            <span className="code-col">one method per txn</span>,
            <>
              A public no-arg method per <C>txn</C> declaration (name sanitized from its string literal, e.g.{" "}
              <C>"coffee"</C> → <C>coffeeTxn</C>). Statements become <C>lload</C>/<C>lstore</C> on local{" "}
              <C>long</C> slots plus calls into <C>Ledger</C>.
            </>,
          ],
          [
            <span className="code-col">run()</span>,
            <>
              Calls every transaction method in declaration order, then calls <C>ledger.report(...)</C> and
              returns the trial balance <C>String</C>.
            </>,
          ],
          [
            <span className="code-col">main(String[])</span>,
            <>
              Constructs the class and prints <C>run()</C>'s output — the class is directly runnable with{" "}
              <C>java -cp ... ClassName</C>, and equally usable as a library from any other Java program.
            </>,
          ],
        ]}
      />

      <h2>
        The runtime shim — <C>balanc.runtime.Ledger</C>
      </h2>
      <p>
        A small, dependency-free Java class (no framework of any kind) that every generated class links against.
        It carries all of the language's genuinely data-dependent logic:
      </p>
      <CodeBlock lang="java" code={LEDGER_API} />
      <p>
        How compiled code calls it: <C>ledger.post(accountIndex, delta)</C> for every debit/credit/absorb,{" "}
        <C>Ledger.checkedSplitRemainder(total, amount)</C> for <C>split</C>,{" "}
        <C>Ledger.splitRatio(total, p, q)</C> for <C>split_ratio</C>, and <C>ledger.report(...)</C> once, from{" "}
        <C>run()</C>.
      </p>

      <h2>Building, packaging, and calling it</h2>
      <Terminal
        lines={[
          { type: "comment", text: "1. Emit the class alongside the interpreter's own run" },
          { type: "cmd", text: "cargo run -- examples/coffee.bal --emit-jvm Coffee.class" },
          { type: "out", text: "" },
          { type: "comment", text: "2. Compile the (fixed, unchanging) runtime shim once" },
          { type: "cmd", text: "javac -d out runtime/balanc/runtime/Ledger.java" },
          { type: "cmd", text: "cp Coffee.class out/" },
          { type: "out", text: "" },
          { type: "comment", text: "3. Run it directly …" },
          { type: "cmd", text: "java -Xverify:all -cp out Coffee" },
          { type: "out", text: "" },
          { type: "comment", text: "… or package it as a plain, framework-free jar and call it from any Java program" },
          { type: "cmd", text: "jar cf coffee.jar -C out Coffee.class -C out balanc" },
        ]}
      />
      <p>
        Packaged this way, a generated class is wireable into a Spring Boot app (or anything else) exactly like
        any other library jar — <C>@Bean</C> or plain <C>new</C> — because the generated API and the runtime
        shim have no dependency beyond the JDK itself.
      </p>

      <h2>Proving the two backends agree</h2>
      <p>
        A differential test suite compiles each example fixture both ways and asserts the JVM path's stdout is
        byte-for-byte identical to the interpreter's report, using <C>javac</C>/<C>java</C>/<C>jar</C>/
        <C>javap</C> directly — no test-only dependency either. It also runs with <C>-Xverify:all</C> (the class
        must pass real JVM bytecode verification), checks a <C>javap -c</C> disassembly for a legible method
        shape, and compiles a tiny external Java program against a packaged jar to confirm the output is usable
        as an ordinary library, not just as a standalone class.
      </p>
    </DocPage>
  );
}
