//! Untyped syntax tree. `resolve` turns each binding/reference name, and each
//! account/currency name, into an id; `typeck` then checks the result and produces the
//! `T`-prefixed IR `eval` runs.
//!
//! Grammar (Slice 2): a module is a sequence of `currency` and `account` declarations
//! plus `txn` declarations, in any order. A transaction body is a sequence of `let`
//! bindings and `debit` statements (D-012, Slice 1). `credit` is the only way to
//! introduce a `Money` value (D-005) and can appear either inline in a `debit`'s
//! argument or as a `let`'s right-hand side; a bound name can then flow into a `debit`
//! or into another `let` (a move). There is deliberately no statement form for a bare,
//! unconsumed `credit` or `Var` — every `Money` value is produced only where it can
//! immediately be bound or consumed.
//!
//! A decimal literal is kept as raw text (`DecimalLiteral`) rather than lowered to an
//! `Amount` here, because lowering needs a scale, and scale is a per-currency property
//! not known until the literal's account (and hence currency) resolves (D-024).

use crate::span::Span;

pub struct Module {
    pub currencies: Vec<CurrencyDecl>,
    pub accounts: Vec<AccountDecl>,
    pub txns: Vec<TxnDecl>,
}

/// `currency NAME { scale = N }` — declares a currency and its fixed-point scale.
pub struct CurrencyDecl {
    pub name: String,
    pub name_span: Span,
    pub scale: u32,
    pub scale_span: Span,
    pub span: Span,
}

/// `account PATH { currency = CURRENCY }` — binds an account path to a currency
/// (D-022, D-023). The `{ field = value }` block mirrors `CurrencyDecl`'s own shape
/// rather than a `:` connector, which is ambiguous with `AccountPath`'s own `:`
/// separator (D-025). Minimal through Slice 2: no account kind or normal-balance sign
/// yet (Slice 5 adds those as more fields in the same block).
pub struct AccountDecl {
    pub path: AccountPath,
    pub currency: String,
    pub currency_span: Span,
    pub span: Span,
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
    /// `credit(ACCOUNT, AMOUNT)` — introduces a fresh linear value (T-Credit). The
    /// amount is still a raw literal; `typeck` lowers it once it knows `ACCOUNT`'s
    /// currency's scale.
    Credit { account: AccountPath, amount: DecimalLiteral, span: Span },
    /// A reference to a previously `let`-bound name.
    Var { name: String, span: Span },
}

/// A decimal literal's raw text, e.g. `"45.00"` or `"45"` — lexer-validated (digits,
/// at most one `.`, at least one fraction digit if a `.` is present) but not yet
/// lowered to an `Amount` (needs a scale, see module doc comment).
pub struct DecimalLiteral {
    pub text: String,
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
