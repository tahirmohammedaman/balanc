//! Untyped syntax tree. `resolve` turns each binding/reference name into a `SymbolId`;
//! `typeck` then checks the result and produces the `T`-prefixed IR `eval` runs.
//!
//! Grammar (Slice 1, D-012): a transaction body is a sequence of `let` bindings and
//! `debit` statements. `credit` is the only way to introduce a `Money` value (D-005)
//! and can appear either inline in a `debit`'s argument or as a `let`'s right-hand
//! side; a bound name can then flow into a `debit` or into another `let` (a move).
//! There is deliberately no statement form for a bare, unconsumed `credit` or `Var` —
//! every `Money` value is produced only where it can immediately be bound or consumed.

use crate::amount::Amount;
use crate::span::Span;

pub struct Module {
    pub txns: Vec<TxnDecl>,
}

pub struct TxnDecl {
    pub name: String,
    pub stmts: Vec<Stmt>,
    /// Covers the whole `txn "..." { ... }` block.
    pub span: Span,
}

pub enum Stmt {
    /// `let NAME = MoneyExpr;` — binds a fresh `Money` value. `name_span` is exactly
    /// the binding's span, which is what `E_DROPPED` points at if it's never consumed.
    Let { name: String, name_span: Span, value: MoneyExpr, span: Span },
    /// `debit(ACCOUNT, MoneyExpr);` — consumes the value MoneyExpr produces.
    Debit { account: AccountPath, value: MoneyExpr, span: Span },
}

pub enum MoneyExpr {
    /// `credit(ACCOUNT, AMOUNT)` — introduces a fresh linear value (T-Credit).
    Credit { account: AccountPath, amount: Amount, span: Span },
    /// A reference to a previously `let`-bound name.
    Var { name: String, span: Span },
}

pub struct AccountPath {
    pub segments: Vec<String>,
    pub span: Span,
}

impl std::fmt::Display for AccountPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.segments.join(":"))
    }
}
