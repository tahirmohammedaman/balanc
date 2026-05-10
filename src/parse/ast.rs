//! Untyped syntax tree. `resolve` turns each binding/reference name, and each
//! account/currency name, into an id; `typeck` then checks the result and produces the
//! `T`-prefixed IR `eval` runs.
//!
//! Grammar (Slice 4): a module is a sequence of `currency`, `account`, and `rate`
//! declarations plus `txn` declarations, in any order. A transaction body is a
//! sequence of `let`, `convert`, `debit`, `absorb`, `split`, and `split_ratio`
//! statements. `credit` and `merge` are the only ways to introduce a `Money` value
//! (D-005, and Slice 4's `merge` — T-Merge) and can appear either inline in a
//! `debit`/`convert`/`split`/`split_ratio`/`merge` argument or as a `let`'s
//! right-hand side; a bound name can then flow into any of those, or another `let` (a
//! move). `convert` is the only way to introduce a `Residue` value (D-026); a bound
//! residue name can flow into `absorb` or another `let`. There is deliberately no
//! statement form for a bare, unconsumed `credit`, `convert`, `merge`, `split`,
//! `split_ratio`, or `Var` — every value is produced only where it can immediately be
//! bound or consumed.
//!
//! A decimal literal is kept as raw text (`DecimalLiteral`) rather than lowered to an
//! `Amount` here, because lowering needs a scale, and scale is a per-currency property
//! not known until the literal's account (and hence currency) resolves (D-024). A
//! rate's literal is lowered the same way, in `resolve` (its precision needs no
//! currency lookup — see `resolve::resolve_rates`).

use crate::span::Span;

pub struct Module {
    pub currencies: Vec<CurrencyDecl>,
    pub accounts: Vec<AccountDecl>,
    pub rates: Vec<RateDecl>,
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

/// `rate NAME from A to B = VALUE round down;` (D-027 — flat statement; `round down`
/// is required syntax even though `down` is currently the only legal value, per
/// D-016's auditability rationale carried through by D-027).
pub struct RateDecl {
    pub name: String,
    pub name_span: Span,
    pub from: String,
    pub from_span: Span,
    pub to: String,
    pub to_span: Span,
    pub value: DecimalLiteral,
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
    /// `let (PRIMARY, RESIDUAL) = convert(MoneyExpr, RATE);` — consumes the value
    /// `MoneyExpr` produces, binds `PRIMARY : Money<B>` and `RESIDUAL : Residue<B>`
    /// (D-026), where `B` is `RATE`'s declared `to` currency.
    Convert {
        primary_name: String,
        primary_span: Span,
        residual_name: String,
        residual_span: Span,
        money: MoneyExpr,
        rate: String,
        rate_span: Span,
        span: Span,
    },
    /// `debit(ACCOUNT, MoneyExpr);` — consumes the value MoneyExpr produces.
    Debit { account: AccountPath, value: MoneyExpr, span: Span },
    /// `absorb(RESIDUE, ACCOUNT);` — consumes a `Residue` bound by a `Convert`
    /// statement, posting its whole-minor-units to `ACCOUNT` (D-028).
    Absorb { residue_name: String, residue_span: Span, account: AccountPath, span: Span },
    /// `let (A, B) = split(MoneyExpr, AMOUNT);` — consumes the value `MoneyExpr`
    /// produces, binds `A : Money<C>` (worth exactly `AMOUNT`) and `B : Money<C>`
    /// (worth the remainder) — T-Split (D-034). `AMOUNT` exceeding the input's actual
    /// value is a real, only-known-at-runtime error (`E_UNBALANCED`), not a parse-time
    /// one — same reason `credit`'s amount is a raw literal, one level further removed.
    Split {
        a_name: String,
        a_span: Span,
        b_name: String,
        b_span: Span,
        money: MoneyExpr,
        amount: DecimalLiteral,
        span: Span,
    },
    /// `let (A, B) = split_ratio(MoneyExpr, P, Q);` — T-SplitRatio (D-015, D-035): `P`
    /// and `Q` are non-negative whole-number weights (not `DecimalLiteral` — a ratio's
    /// precision doesn't need fractional weights, D-035), and the input's value is
    /// allocated `A : Money<C>`, `B : Money<C>` by largest-remainder allocation in
    /// ratio `P : Q`, ties broken toward `A`. Always exact — no runtime side condition,
    /// unlike `Split`.
    SplitRatio {
        a_name: String,
        a_span: Span,
        b_name: String,
        b_span: Span,
        money: MoneyExpr,
        a_weight: u32,
        a_weight_span: Span,
        b_weight: u32,
        b_weight_span: Span,
        span: Span,
    },
}

pub enum MoneyExpr {
    /// `credit(ACCOUNT, AMOUNT)` — introduces a fresh linear value (T-Credit). The
    /// amount is still a raw literal; `typeck` lowers it once it knows `ACCOUNT`'s
    /// currency's scale.
    Credit { account: AccountPath, amount: DecimalLiteral, span: Span },
    /// A reference to a previously `let`-bound name.
    Var { name: String, span: Span },
    /// `merge(MoneyExpr, MoneyExpr)` — T-Merge. The first `MoneyExpr` produced by this
    /// grammar that nests (a `credit`'s arguments are an `AccountPath` and a
    /// `Decimal`, never another `MoneyExpr`) — see `parse::Parser`'s depth cap.
    Merge { a: Box<MoneyExpr>, b: Box<MoneyExpr>, span: Span },
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
