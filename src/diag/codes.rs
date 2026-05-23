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
    /// A decimal literal (an amount, a rate value, or a currency's scale applied to
    /// one) is too large in magnitude for the fixed-point `i64` representation this
    /// language is built on — lexer-validated digit *shape* says nothing about
    /// magnitude, so this is checked where the literal is lowered, not at lex time.
    AmountOutOfRange,
    /// A `debit`'s money value and its target account have different currencies
    /// (Slice 2, T-Debit's `acct : Account<C>` premise); also raised when `convert`'s
    /// input doesn't match its rate's `from` currency, or `absorb`'s residue doesn't
    /// match its target account's currency (Slice 3).
    CurrencyMismatch,
    /// A `convert`/`absorb` names a rate with no matching `rate` declaration.
    UndeclaredRate,
    /// Two `rate` declarations use the same name.
    DuplicateRate,
    /// A `debit` (or `convert`'s money argument) names a `Var` that resolved to a
    /// `Residue`, not `Money` — use `absorb`, not `debit`, to discharge a residue.
    ExpectedMoney,
    /// An `absorb` names a `Var` that resolved to `Money`, not a `Residue` — use
    /// `debit`, not `absorb`, to discharge ordinary money.
    ExpectedResidue,
    /// A `let`-bound `Money` value is never consumed by a `debit` — Δ is non-empty at
    /// the end of the transaction body (invariant 1).
    Dropped,
    /// A `Money` value is consumed a second time (invariant 1).
    Reused,
    /// A `split(e, n)` names an `n` exceeding `e`'s actual value (invariant 3,
    /// T-Split's side condition, D-034) — checked in `eval`, since `e`'s concrete
    /// amount generally isn't known until then. The one way invariant 3's balance
    /// premise is still a real, computed check rather than falling out structurally
    /// from linearity (T-Merge's arithmetic and T-SplitRatio's largest-remainder
    /// allocation are both exact by construction and can't violate it).
    Unbalanced,
    /// A `merge`/`split`/`split_ratio` chain nests a `MoneyExpr` more than
    /// `parse::MAX_MONEY_EXPR_DEPTH` levels deep (Fix checkpoint B, D-031: no
    /// recursive production existed to cap until Slice 4's `merge`).
    ExprTooDeep,
    /// A `split_ratio(e, p, q)` names weights `p` and `q` that are both zero — the
    /// ratio `0 : 0` doesn't determine an allocation (T-SplitRatio's side condition).
    ZeroRatio,
    /// An `account` declaration's `kind` field names something other than one of the
    /// five closed values (`asset`/`liability`/`equity`/`income`/`expense`, Slice 5).
    /// `kind` is a bare `Ident`, not a set of keywords (D-036), so this closed-set
    /// membership check happens here, in `resolve`, the same place `E_UNKNOWN_CURRENCY`
    /// checks a currency name against the (open, user-declared) currency table.
    UnknownAccountKind,
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
            Code::AmountOutOfRange => "E_AMOUNT_OUT_OF_RANGE",
            Code::CurrencyMismatch => "E_CURRENCY_MISMATCH",
            Code::UndeclaredRate => "E_UNDECLARED_RATE",
            Code::DuplicateRate => "E_DUPLICATE_RATE",
            Code::ExpectedMoney => "E_EXPECTED_MONEY",
            Code::ExpectedResidue => "E_EXPECTED_RESIDUE",
            Code::Dropped => "E_DROPPED",
            Code::Reused => "E_REUSED",
            Code::Unbalanced => "E_UNBALANCED",
            Code::ExprTooDeep => "E_EXPR_TOO_DEEP",
            Code::ZeroRatio => "E_ZERO_RATIO",
            Code::UnknownAccountKind => "E_UNKNOWN_ACCOUNT_KIND",
        }
    }

    /// Fixed, general guidance for this class of error — printed as the trailing
    /// `= note:` line of every diagnostic with this code (Slice 6), regardless of the
    /// specific message above it.
    pub fn note(self) -> &'static str {
        match self {
            Code::LexUnexpectedChar => "remove or replace the invalid character",
            Code::ParseUnexpectedToken => "check the grammar for what's expected at this point",
            Code::ParseUnexpectedEof => {
                "the file ended before a construct was closed — check for a missing '}' or ';'"
            }
            Code::UnboundName => {
                "check the spelling, or add a 'let' binding for this name earlier in the transaction"
            }
            Code::UnknownCurrency => {
                "declare this currency with a 'currency NAME { scale = N }' block before referencing it"
            }
            Code::DuplicateCurrency => "each currency name may only be declared once",
            Code::DuplicateAccount => "each account path may only be declared once",
            Code::UndeclaredAccount => {
                "declare this account with an 'account PATH { ... }' block before crediting or debiting it"
            }
            Code::TooManyFractionDigits => {
                "round the literal to the currency's declared scale, or declare the currency with a larger scale"
            }
            Code::AmountOutOfRange => {
                "this magnitude does not fit the fixed-point representation this language uses"
            }
            Code::CurrencyMismatch => {
                "money only moves between accounts, rates, or residues that share the same currency — use 'convert' to cross currencies"
            }
            Code::UndeclaredRate => {
                "declare this rate with a 'rate NAME from A to B = VALUE round down;' statement before referencing it"
            }
            Code::DuplicateRate => "each rate name may only be declared once",
            Code::ExpectedMoney => {
                "this binding holds a conversion residue — discharge it with 'absorb', not 'debit'"
            }
            Code::ExpectedResidue => {
                "this binding holds ordinary money — discharge it with 'debit', not 'absorb'"
            }
            Code::Dropped => {
                "every money value bound in a transaction must be consumed exactly once — consume it with 'debit', 'absorb', 'split', 'split_ratio', or 'merge'"
            }
            Code::Reused => {
                "every money value bound in a transaction may only be consumed once — bind a new value instead of reusing this one"
            }
            Code::Unbalanced => {
                "this operation would create or destroy money — check the amount against what's actually available"
            }
            Code::ExprTooDeep => {
                "flatten this expression — deeply nested 'merge' chains are rejected before they risk overflowing the parser's stack"
            }
            Code::ZeroRatio => "at least one of split_ratio's two weights must be nonzero",
            Code::UnknownAccountKind => {
                "kind must be one of: asset, liability, equity, income, expense"
            }
        }
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
