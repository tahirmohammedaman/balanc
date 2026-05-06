//! Stable diagnostic codes. One entry per user-facing error; never reused for a
//! different meaning once released.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// A character does not begin any valid token.
    LexUnexpectedChar,
    /// A numeric literal has more than two fractional digits (scale is fixed at 2
    /// until per-currency scales land in Slice 2).
    LexTooManyFractionDigits,
    /// The parser found a token it cannot use at this point in the grammar.
    ParseUnexpectedToken,
    /// End of input reached while a construct (e.g. `txn { ... }`) was still open.
    ParseUnexpectedEof,
    /// A `Var` refers to a name with no `let` binding in scope.
    UnboundName,
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
            Code::LexTooManyFractionDigits => "E_LEX_TOO_MANY_FRACTION_DIGITS",
            Code::ParseUnexpectedToken => "E_PARSE_UNEXPECTED_TOKEN",
            Code::ParseUnexpectedEof => "E_PARSE_UNEXPECTED_EOF",
            Code::UnboundName => "E_UNBOUND_NAME",
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
