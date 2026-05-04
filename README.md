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

Early. Scaffolding only.

The first target is a tree-walking interpreter. A JVM bytecode backend (via ASM),
consuming the same typed IR, comes later.

## Build

Requires a Rust toolchain.

```
cargo build
cargo test
cargo run -- examples/coffee.bal
```

## Layout

```
src/        compiler and interpreter
examples/   sample programs, also used as end-to-end test inputs
tests/      end-to-end tests and diagnostic snapshots
```
