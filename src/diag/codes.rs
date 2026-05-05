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
    /// A transaction's debits and credits do not sum equal.
    Unbalanced,
}

impl Code {
    pub fn as_str(self) -> &'static str {
        match self {
            Code::LexUnexpectedChar => "E_LEX_UNEXPECTED_CHAR",
            Code::LexTooManyFractionDigits => "E_LEX_TOO_MANY_FRACTION_DIGITS",
            Code::ParseUnexpectedToken => "E_PARSE_UNEXPECTED_TOKEN",
            Code::ParseUnexpectedEof => "E_PARSE_UNEXPECTED_EOF",
            Code::Unbalanced => "E_UNBALANCED",
        }
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
