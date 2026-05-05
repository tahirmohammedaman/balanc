//! Untyped syntax tree for Slice 0. `typeck` (Slice 1) will produce a parallel
//! `T`-prefixed IR from this; nothing here carries type information.

use crate::amount::Amount;
use crate::span::Span;

pub struct Module {
    pub txns: Vec<TxnDecl>,
}

pub struct TxnDecl {
    pub name: String,
    pub legs: Vec<Leg>,
    /// Covers the whole `txn "..." { ... }` block — this is the span `E_UNBALANCED`
    /// points at.
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegKind {
    Debit,
    Credit,
}

pub struct Leg {
    pub kind: LegKind,
    pub account: AccountPath,
    pub amount: Amount,
    pub span: Span,
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
