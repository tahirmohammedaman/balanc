# balanc

A statically typed language for double-entry bookkeeping, in which the ledger
invariants are enforced by the type system instead of checked at runtime.

Money is a **linear** resource: every money value must be consumed exactly once.

- Dropping a value is a compile error — money would vanish.
- Using a value twice is a compile error — money would be duplicated.
- A transaction whose debits and credits do not agree does not compile.

Money is parameterized by currency, so `Money<ETB>` and `Money<USD>` are distinct
types that cannot be combined; crossing between them requires an explicit `Rate`
value at the conversion site. Values are only produced and consumed through
total-preserving operations — `credit`, `debit`, `split`, `merge` — and a bare number
cannot become money inside a transaction body.

## Status

A tree-walking interpreter (`balanc::run`) and a JVM bytecode backend
(`balanc::backend::jvm`) both consume the same typed IR (`typeck::TModule`). The JVM
backend hand-writes `.class` bytes directly in Rust — no ASM, no bytecode-manipulation
library — and packages the result as an ordinary, framework-free `.jar`.

## Build

Requires a Rust toolchain. The JVM backend's tests and `--emit-jvm` additionally need
a JDK (17+) with `javac`/`java`/`jar` on `PATH`.

```
cargo build
cargo test
cargo run -- examples/coffee.bal
cargo run -- examples/coffee.bal --emit-jvm Coffee.class
```

## Layout

```
src/        compiler, interpreter, and the JVM backend (src/backend/jvm/)
runtime/    the JVM backend's small Java runtime shim (balanc.runtime.Ledger)
examples/   sample programs, also used as end-to-end test inputs
tests/      end-to-end tests, diagnostic snapshots, and JVM backend proof tests
```
