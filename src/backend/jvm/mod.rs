//! The JVM bytecode backend (PLAN.md Slice 7, D-041/D-042/D-043). Consumes the same
//! `typeck::TModule` the interpreter (`eval`) does and emits a single hand-written
//! `.class` file — no ASM, no second language in the *build* (D-041): the only Java
//! source in this project is `runtime/balanc/runtime/Ledger.java`, a small, fixed,
//! `javac`-compiled runtime shim (D-043's "framework-free" API surface), not
//! generated per module.
//!
//! - `pool` — the constant pool (JVMS §4.4).
//! - `code` — a `Code` attribute builder; every method this backend emits is
//!   straight-line, so it never needs a `StackMapTable`.
//! - `class` — assembles a complete `.class` file from a pool plus fields/methods.
//! - `codegen` — walks a `TModule` and drives the above to produce one class's bytes.

pub mod class;
pub mod code;
pub mod codegen;
pub mod pool;

pub use codegen::compile;
