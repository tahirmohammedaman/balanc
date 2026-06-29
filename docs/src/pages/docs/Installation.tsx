import { Link } from "react-router-dom";
import { DocPage } from "../../components/DocPage";
import { C, Callout, DocTable } from "../../components/Prose";
import { Terminal } from "../../components/code/CodeBlock";

export function Installation() {
  return (
    <DocPage
      title="Installation"
      lede="Three ways to get balanc onto your PATH — an install script, a manual download, or building it yourself. All three end the same way: one self-contained binary, nothing to install alongside it."
    >
      <h2>Install script</h2>
      <p>
        Fetches the right archive for your platform from the{" "}
        <a href="https://github.com/tahirmohammedaman/balanc/releases/latest" target="_blank" rel="noreferrer">
          latest GitHub release
        </a>
        , verifies its checksum, and unpacks <C>balanc</C> into <C>~/.cargo/bin</C> — the same directory{" "}
        <C>rustup</C>/<C>cargo install</C> use, so it's already on <C>PATH</C> for anyone with a Rust toolchain
        installed.
      </p>
      <p>macOS / Linux:</p>
      <Terminal
        lines={[
          {
            type: "cmd",
            text: "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/tahirmohammedaman/balanc/releases/latest/download/balanc-installer.sh | sh",
          },
        ]}
      />
      <p>Windows (PowerShell):</p>
      <Terminal
        lines={[
          {
            type: "cmd",
            text: 'powershell -ExecutionPolicy Bypass -c "irm https://github.com/tahirmohammedaman/balanc/releases/latest/download/balanc-installer.ps1 | iex"',
          },
        ]}
      />

      <h2>Download a release directly</h2>
      <p>
        Each{" "}
        <a href="https://github.com/tahirmohammedaman/balanc/releases" target="_blank" rel="noreferrer">
          GitHub release
        </a>{" "}
        publishes a prebuilt archive per platform, each with a <C>.sha256</C> checksum file alongside it:
      </p>
      <DocTable
        columns={[{ header: "Target" }, { header: "Platform" }]}
        rows={[
          [<span className="code-col">x86_64-unknown-linux-gnu</span>, "Linux, x86-64"],
          [<span className="code-col">aarch64-unknown-linux-gnu</span>, "Linux, ARM64"],
          [<span className="code-col">x86_64-apple-darwin</span>, "macOS, Intel"],
          [<span className="code-col">aarch64-apple-darwin</span>, "macOS, Apple Silicon"],
          [<span className="code-col">x86_64-pc-windows-msvc</span>, "Windows, x86-64"],
        ]}
      />
      <p>
        Download the archive matching your platform, verify it against its checksum, extract it, and place the{" "}
        <C>balanc</C> binary somewhere on your <C>PATH</C>:
      </p>
      <Terminal
        lines={[
          { type: "comment", text: "example: Linux x86-64" },
          { type: "cmd", text: "shasum -a 256 -c balanc-x86_64-unknown-linux-gnu.tar.xz.sha256" },
          { type: "cmd", text: "tar -xf balanc-x86_64-unknown-linux-gnu.tar.xz" },
          { type: "cmd", text: "mv balanc-x86_64-unknown-linux-gnu/balanc ~/.local/bin/" },
        ]}
      />

      <h2>Build from source</h2>
      <p>
        Needs a Rust toolchain (stable). Clone the repository, then either build a binary or install it
        directly:
      </p>
      <Terminal
        lines={[
          { type: "comment", text: "produces target/release/balanc" },
          { type: "cmd", text: "cargo build --release" },
          { type: "out", text: "" },
          { type: "comment", text: "…or install straight onto PATH, alongside cargo itself" },
          { type: "cmd", text: "cargo install --path ." },
        ]}
      />
      <p>
        The interpreter alone only needs Rust. <C>--emit-jvm</C> additionally needs a JDK (17+) with{" "}
        <C>javac</C>/<C>java</C>/<C>jar</C> on <C>PATH</C> — see{" "}
        <Link to="/docs/architecture/jvm-backend">the JVM backend reference</Link>.
      </p>

      <h2>Verify</h2>
      <Terminal lines={[{ type: "cmd", text: "balanc --version" }, { type: "out", text: "balanc 0.1.0" }]} />

      <Callout title="Zero runtime dependencies">
        <p>
          <C>balanc</C> is a single binary with no bundled runtime or framework — copy it anywhere, commit it to
          a CI image, or vendor it in another project's tooling directory. There is nothing else to install
          alongside it unless you're using <C>--emit-jvm</C>, in which case the JDK is the only extra piece.
        </p>
      </Callout>
    </DocPage>
  );
}
