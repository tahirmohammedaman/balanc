//! Stable diagnostic codes. One entry per user-facing error; never reused for a
//! different meaning once released.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// A character does not begin any valid token.
    LexUnexpectedChar,
    /// The parser found a token it cannot use at this point in the grammar.
    ParseUnexpectedToken,
    /// End of input reached while a construct (e.g. `txn { ... }`) was still open.
    ParseUnexpectedEof,
    /// A `Var` refers to a name with no `let` binding in scope.
    UnboundName,
    /// An `account` declaration names a currency with no matching `currency`
    /// declaration.
    UnknownCurrency,
    /// Two `currency` declarations use the same name.
    DuplicateCurrency,
    /// Two `account` declarations name the same account path.
    DuplicateAccount,
    /// A `credit`/`debit` names an account path with no `account` declaration —
    /// pulled forward from Slice 5 into Slice 2 (D-022), since currency inference
    /// needs every referenced account to already resolve to a declared currency.
    UndeclaredAccount,
    /// A decimal literal has more fraction digits than its resolved currency's scale
    /// allows. Checked at `typeck` time (D-024), once the literal's currency is known
    /// — scale is a per-currency property starting Slice 2, not a language constant.
    TooManyFractionDigits,
    /// A `debit`'s money value and its target account have different currencies
    /// (Slice 2, T-Debit's `acct : Account<C>` premise).
    CurrencyMismatch,
    /// A `let`-bound `Money` value is never consumed by a `debit` — Δ is non-empty at
    /// the end of the transaction body (invariant 1).
    Dropped,
    /// A `Money` value is consumed a second time (invariant 1).
    Reused,
    /// A transaction's debits and credits do not sum equal (invariant 3). Reserved:
    /// unreachable through Slice 3 given the grammar (every `credit`-introduced value
    /// is consumed by exactly one `debit`, with no arithmetic to unbalance the sums in
    /// between) — expected to become reachable once `split`/`merge` land in Slice 4.
    Unbalanced,
}

impl Code {
    pub fn as_str(self) -> &'static str {
        match self {
            Code::LexUnexpectedChar => "E_LEX_UNEXPECTED_CHAR",
            Code::ParseUnexpectedToken => "E_PARSE_UNEXPECTED_TOKEN",
            Code::ParseUnexpectedEof => "E_PARSE_UNEXPECTED_EOF",
            Code::UnboundName => "E_UNBOUND_NAME",
            Code::UnknownCurrency => "E_UNKNOWN_CURRENCY",
            Code::DuplicateCurrency => "E_DUPLICATE_CURRENCY",
            Code::DuplicateAccount => "E_DUPLICATE_ACCOUNT",
            Code::UndeclaredAccount => "E_UNDECLARED_ACCOUNT",
            Code::TooManyFractionDigits => "E_TOO_MANY_FRACTION_DIGITS",
            Code::CurrencyMismatch => "E_CURRENCY_MISMATCH",
            Code::Dropped => "E_DROPPED",
            Code::Reused => "E_REUSED",
            Code::Unbalanced => "E_UNBALANCED",
        }
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
