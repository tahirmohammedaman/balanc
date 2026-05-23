//! Inference rules for the balanc type system, written before the code that
//! implements them (ground rule). `Γ` is the linear context: the set of not-yet-
//! consumed bindings in scope — either `Money` or `Residue` (Slice 3, D-026) — each
//! tagged with the currency it was produced at (Slice 2).
//!
//! ```text
//!                 acct : Account<C>       Γ ⊢ n : Amount
//! (T-Credit)  ───────────────────────────────────────────────
//!             Γ ⊢ credit(acct, n) : Money<C>     introduces a fresh linear value
//!
//!             Γ ⊢ e : Money<C>     acct : Account<C>
//! (T-Debit)   ────────────────────────────────────────
//!             Γ ⊢ debit(acct, e) : Effect     consumes e
//!
//!             Γ ⊢ e : Money<A>     rate : Rate<A, B>
//! (T-Convert) ─────────────────────────────────────────────────────────────────
//!             Γ ⊢ let (p, r) = convert(e, rate) : Money<B> ⊗ Residue<B>
//!                 consumes e; binds p : Money<B>, r : Residue<B>
//!
//!             Γ ⊢ x : Residue<C>     acct : Account<C>
//! (T-Absorb)  ──────────────────────────────────────────
//!             Γ ⊢ absorb(x, acct) : Effect     consumes x
//!
//!             Γ ⊢ e : Money<C>     n : Amount
//! (T-Split)   ─────────────────────────────────────────────────────
//!             Γ ⊢ let (a, b) = split(e, n) : Money<C> ⊗ Money<C>
//!                 consumes e; binds a : Money<C> (= n), b : Money<C> (= amount(e) - n)
//!                 side condition: 0 ≤ n ≤ amount(e) — unlike every premise above,
//!                 `amount(e)` is a runtime quantity, not something `typeck` can see
//!                 (Slice 0's boundary: types track currency and linearity, never
//!                 magnitude), so this is checked in `eval`, not here (D-034). A
//!                 violation is `E_UNBALANCED`: handing out more than `e` holds would
//!                 manufacture money from nothing, exactly what invariant 3 forbids.
//!
//!             Γ ⊢ e : Money<C>     p, q : Nat     p + q > 0
//! (T-SplitRatio) ───────────────────────────────────────────────────────
//!             Γ ⊢ let (a, b) = split_ratio(e, p, q) : Money<C> ⊗ Money<C>
//!                 consumes e; binds a, b to a largest-remainder allocation of
//!                 amount(e) in ratio p : q (D-015), ties broken toward a. Exact by
//!                 construction (a + b always sums to amount(e)) — no side condition,
//!                 unlike T-Split, other than the literal-checkable p + q > 0.
//!
//!             Γ ⊢ a : Money<C>     Γ ⊢ b : Money<C>
//! (T-Merge)   ─────────────────────────────────────────
//!             Γ ⊢ merge(a, b) : Money<C>     consumes a, then b, from the same Γ
//!                 (Γ₁ ⊎ Γ₂'s disjointness requirement falls out for free this way:
//!                 consuming `a` then `b` in sequence from one Γ already rejects
//!                 `merge(m, m)` as an ordinary second consumption — E_REUSED, no new
//!                 mechanism needed)
//!
//!             Γ ⊢ body ⇒ Δ        Δ = ∅
//! (T-Txn)     ─────────────────────────────
//!             ⊢ txn { body } : Txn
//! ```
//!
//! `merge` is `MoneyExpr`, not a statement (unlike `split`/`split_ratio`, which — like
//! `convert` — produce a pair and so need `let (a, b) = ...`'s two binding sites):
//! `merge(a, b)` yields a single `Money`, so it fits the same grammar slot as `credit`
//! or a bare variable, usable as a `let`'s right-hand side or inline in `debit`. This
//! is also the first place `MoneyExpr` actually nests (`merge`'s own arguments are
//! `MoneyExpr`s, so `merge(credit(...), merge(...))` parses) — Fix checkpoint B found
//! no recursive production to cap because none existed yet; `parse::Parser` now caps
//! nesting depth with `E_EXPR_TOO_DEEP` now that one does.
//!
//! `Δ = ∅` is invariant 1 (linear use); `Context::finish_txn` below is what checks it,
//! reporting every still-live binding — `Money` or `Residue` alike — as `E_DROPPED`.
//! `Context::consume` is what "consumes e"/"consumes x" means operationally: remove it
//! from Γ, or `E_REUSED` if it is already gone.
//!
//! The balance premise (invariant 3) was, through Slice 2, discharged *structurally*
//! rather than by an explicit sum: the grammar had no way to create a `Money` value
//! except `credit`, no way to destroy one except `debit`, and no arithmetic in
//! between — so `Δ = ∅` alone forced every credited amount to reach exactly one
//! same-currency debit. `convert` breaks that argument (D-029): a `credit`ed value can
//! now leave its currency's own books entirely, converted into a different currency's
//! `Money`/`Residue` pair, so a single currency's nominal totals no longer have to net
//! to zero within a transaction — which is correct once conversion exists, not a
//! regression. `Δ = ∅` still guarantees nothing is dropped; it just no longer implies
//! each currency balances independently. An explicit cross-currency computed check
//! (base-currency triangulation) is out of scope. `split` (Slice 4, D-034) is where
//! `Code::Unbalanced` stops being reserved: `Δ = ∅` still guarantees every value is
//! consumed exactly once, but it says nothing about the *magnitude* `split` hands out
//! for each half, so that specific arithmetic side condition needs a real, computed
//! check for the first time — see T-Split above.
//!
//! The `acct : Account<C>` premise is checked (Slice 2, D-022): every `credit`,
//! `debit`, and (Slice 3) `absorb` names a declared account, resolved to an
//! `AccountId` by `resolve`, whose declared `CurrencyId` `typeck` looks up. `Money<C>`
//! and `Residue<C>` are likewise real: `Context` tags each live binding with its kind
//! (`Money` or `Residue`) and the `CurrencyId` it was produced at, so `T-Debit`'s and
//! `T-Absorb`'s premises — that the value's currency matches the target account's, and
//! that it's the *right kind* of value for the operation — can actually be checked. A
//! currency mismatch is `E_CURRENCY_MISMATCH`; using a `Residue` where `Money` is
//! wanted (or vice versa) is `E_EXPECTED_MONEY` / `E_EXPECTED_RESIDUE`.

use std::collections::HashMap;

use crate::amount::{Amount, LiteralError};
use crate::diag::{Code, Diagnostic};
use crate::parse::ast::DecimalLiteral;
use crate::resolve::{AccountInfo, CurrencyId, CurrencyInfo, RateInfo, SymbolId};
use crate::span::Span;

/// Types in the balanc type system. `Money` and `Residue` (Slice 3, D-026) are the two
/// linear types tracked in Γ — a `Residue` is what `convert` hands back for the
/// fractional amount lost to rounding, and can only be discharged by `absorb`, never
/// by `debit`. `Amount` (a literal argument), `Effect` (what `debit`/`absorb`
/// produce), and `Txn` (a whole transaction) exist here for the record — the grammar
/// does not yet let a program construct a value of `Effect` or `Txn` that would need
/// checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    Money(CurrencyId),
    Residue(CurrencyId),
    Amount,
    Effect,
    Txn,
}

/// Which of the two linear types (`Ty::Money` / `Ty::Residue`) a binding is. Carried
/// alongside the binding rather than folded into `Ty` at the `Context` level so
/// `consume` can report a kind mismatch (`E_EXPECTED_MONEY`/`E_EXPECTED_RESIDUE`) with
/// one error path shared by every call site, instead of each of `debit`/`convert`/
/// `absorb` re-deriving "is this the kind I wanted?" from a `Ty` by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Money,
    Residue,
}

/// A live binding tracks two spans that answer different questions: `binding_span` is
/// the `let`'s (or `convert`'s) own binding site — what `E_DROPPED` points at if this
/// is never consumed — while `currency_span` is wherever this binding's currency was
/// first fixed (the original `credit` or `convert`, forwarded through any number of
/// `let`s) — what an `E_CURRENCY_MISMATCH` cites as "this value's currency was
/// established here". For a binding whose value is a direct `credit`, these coincide
/// only for the very first `let`; forwarding through more `let`s keeps `currency_span`
/// pinned to the original establishing site while `binding_span` moves to each new
/// name.
#[derive(Clone, Copy)]
struct Binding {
    binding_span: Span,
    currency_span: Span,
    currency: CurrencyId,
    kind: BindingKind,
}

/// Γ, scoped to one transaction body (D-013: transactions are closed).
#[derive(Default)]
pub struct Context {
    /// Live bindings, keyed by symbol.
    live: HashMap<SymbolId, Binding>,
    /// Every symbol ever consumed, mapping to the span of that consumption. Kept
    /// after removal from `live` so a second consumption can name both use-sites.
    consumed: HashMap<SymbolId, Span>,
}

/// What a successful `consume` hands back: where the value's currency was
/// established, which currency it is, and which kind of linear value it was —
/// callers check the kind against what the operation actually wanted (D-026).
pub struct Consumed {
    pub currency_span: Span,
    pub currency: CurrencyId,
    pub kind: BindingKind,
}

/// The outcome of `Context::consume`. `NeverBound` is not an error in its own right —
/// see `consume`'s doc comment — callers treat it like a failure (return `None`
/// without checking types further) but push no new diagnostic for it.
pub enum ConsumeResult {
    Ok(Consumed),
    Reused(Diagnostic),
    NeverBound,
}

impl Context {
    /// Adds a fresh live binding. Called once per `let` (or one of `convert`'s two
    /// bindings), after its right-hand side has already been checked (and, if that
    /// side consumed something, after that consumption has already happened) — so a
    /// binding can never see itself. `binding_span` is the binding's own site;
    /// `currency_span` is where this value's currency was established (forwarded from
    /// the right-hand side, or the `convert` statement's own span for a fresh pair).
    pub fn bind(
        &mut self,
        symbol: SymbolId,
        binding_span: Span,
        currency_span: Span,
        currency: CurrencyId,
        kind: BindingKind,
    ) {
        self.live.insert(symbol, Binding { binding_span, currency_span, currency, kind });
    }

    /// (T-Debit's/T-Absorb's "consumes e"/"consumes x") Removes `symbol` from Γ,
    /// returning what it was so the caller can check its kind and currency against
    /// what the consuming operation expects (or forward it, if consumed by another
    /// `let`).
    ///
    /// A name the *resolver* couldn't bind at all is rejected as `E_UNBOUND_NAME`
    /// before typeck ever runs, so that case can't reach here. But a name resolve
    /// *did* bind can still show up here never having entered Γ: if the statement that
    /// was supposed to bind it (a `let` or `convert`) itself failed typeck — e.g. a
    /// malformed literal, or `convert`'s currency mismatch — `bind` is never called
    /// for it, yet a later statement in the same body can still reference it by name.
    /// `ConsumeResult::NeverBound` is that case: the real problem already has its own
    /// diagnostic from whatever failed to bind it, so this doesn't add a second,
    /// confusing one — it just tells the caller to give up on this expression too,
    /// the same as an actual error would.
    pub fn consume(&mut self, symbol: SymbolId, use_span: Span) -> ConsumeResult {
        if let Some(binding) = self.live.remove(&symbol) {
            self.consumed.insert(symbol, use_span);
            ConsumeResult::Ok(Consumed {
                currency_span: binding.currency_span,
                currency: binding.currency,
                kind: binding.kind,
            })
        } else if let Some(&first_span) = self.consumed.get(&symbol) {
            ConsumeResult::Reused(
                Diagnostic::new(Code::Reused, "this value has already been consumed", use_span)
                    .with_secondary("first consumed here", first_span),
            )
        } else {
            ConsumeResult::NeverBound
        }
    }

    /// (T-Txn's `Δ = ∅`) Everything still live at the end of the body was dropped.
    pub fn finish_txn(self) -> Vec<Diagnostic> {
        self.live
            .into_values()
            .map(|binding| {
                let what = match binding.kind {
                    BindingKind::Money => "this money value",
                    BindingKind::Residue => "this conversion residue",
                };
                Diagnostic::new(
                    Code::Dropped,
                    format!("{what} is never consumed"),
                    binding.binding_span,
                )
            })
            .collect()
    }
}

/// Consumes `symbol` from Γ and checks it was the expected kind (`Money` or
/// `Residue`), reporting `E_EXPECTED_MONEY`/`E_EXPECTED_RESIDUE` if not (D-026). Not
/// itself one of the numbered rules above — it's the shared premise every one of
/// them that consumes a value (T-Debit's `e`, T-Convert's `e`, T-Absorb's `x`) needs
/// checked the same way, so it lives here once rather than being re-derived from a
/// bare `Ty` at each call site.
pub fn consume_checked(
    symbol: SymbolId,
    span: Span,
    expected: BindingKind,
    ctx: &mut Context,
    diags: &mut Vec<Diagnostic>,
) -> Option<(Span, CurrencyId)> {
    match ctx.consume(symbol, span) {
        ConsumeResult::Ok(consumed) if consumed.kind == expected => {
            Some((consumed.currency_span, consumed.currency))
        }
        ConsumeResult::Ok(_) => {
            diags.push(kind_mismatch(expected, span));
            None
        }
        ConsumeResult::Reused(d) => {
            diags.push(d);
            None
        }
        ConsumeResult::NeverBound => None,
    }
}

/// (T-Credit) `acct : Account<C>`, `Γ ⊢ n : Amount` ⟹ `Γ ⊢ credit(acct, n) : Money<C>`.
/// `credit` is always where a value's currency is first fixed, so the caller uses its
/// own span as the "currency established here" span — there is nothing earlier to
/// forward from, unlike every other rule below.
pub fn check_credit(
    account_currency: CurrencyId,
    currency: &CurrencyInfo,
    amount: &DecimalLiteral,
    diags: &mut Vec<Diagnostic>,
) -> Option<(Amount, CurrencyId)> {
    let amount = lower_amount(amount, currency.scale, &currency.name, diags)?;
    Some((amount, account_currency))
}

/// (T-Debit's `acct : Account<C>` premise, and T-Absorb's identical one — both
/// operations require the value they discharge to share the target account's
/// currency, checked the same way for either.) The rest of each rule (T-Debit's
/// `Γ ⊢ e : Money<C>`, T-Absorb's `Γ ⊢ x : Residue<C>`) is already established by the
/// caller before this runs, via `typeck_money_expr`/`consume_checked`.
pub fn check_account_currency(
    account: &AccountInfo,
    currencies: &[CurrencyInfo],
    value_currency: CurrencyId,
    value_currency_span: Span,
    use_span: Span,
) -> Result<(), Diagnostic> {
    if value_currency == account.currency {
        Ok(())
    } else {
        Err(currency_mismatch(account, currencies, value_currency, value_currency_span, use_span))
    }
}

/// (T-Convert) `Γ ⊢ e : Money<A>`, `rate : Rate<A, B>` ⟹
/// `Γ ⊢ let (p, r) = convert(e, rate) : Money<B> ⊗ Residue<B>`. Returns `B`, the
/// currency both `p` and `r` are bound at — the caller still does that binding itself
/// (threading Γ is the driver's job, not a rule's).
pub fn check_convert(
    rate: &RateInfo,
    currencies: &[CurrencyInfo],
    money_currency: CurrencyId,
    money_currency_span: Span,
    convert_span: Span,
) -> Result<CurrencyId, Diagnostic> {
    if money_currency == rate.from {
        Ok(rate.to)
    } else {
        Err(rate_currency_mismatch(rate, currencies, money_currency, money_currency_span, convert_span))
    }
}

/// (T-Split's typeck-time half: `n : Amount`.) The side condition `0 ≤ n ≤ amount(e)`
/// is checked in `eval`, not here (D-034) — `amount(e)` is a runtime quantity `typeck`
/// never sees, so all this rule can do ahead of time is lower `n`'s literal at `e`'s
/// currency's scale.
pub fn check_split(
    currency: &CurrencyInfo,
    amount: &DecimalLiteral,
    diags: &mut Vec<Diagnostic>,
) -> Option<Amount> {
    lower_amount(amount, currency.scale, &currency.name, diags)
}

/// (T-SplitRatio's side condition `p + q > 0`.) The allocation itself is exact by
/// construction (largest-remainder, D-015) and computed in `eval`; this is the one
/// static check `typeck` can make ahead of that.
pub fn check_split_ratio(a_weight: u32, b_weight: u32, span: Span) -> Result<(), Diagnostic> {
    if a_weight == 0 && b_weight == 0 {
        Err(Diagnostic::new(
            Code::ZeroRatio,
            "split_ratio's weights are both zero, which doesn't determine an allocation",
            span,
        ))
    } else {
        Ok(())
    }
}

/// (T-Merge) `Γ ⊢ a : Money<C>`, `Γ ⊢ b : Money<C>` ⟹ `Γ ⊢ merge(a, b) : Money<C>`.
pub fn check_merge(
    currencies: &[CurrencyInfo],
    a_currency: CurrencyId,
    a_span: Span,
    b_currency: CurrencyId,
    b_span: Span,
    merge_span: Span,
) -> Result<CurrencyId, Diagnostic> {
    if a_currency == b_currency {
        Ok(a_currency)
    } else {
        Err(merge_currency_mismatch(currencies, a_currency, a_span, b_currency, b_span, merge_span))
    }
}

/// Lowers a decimal literal into an `Amount` at `scale`, reporting
/// `TooManyFractionDigits`/`AmountOutOfRange` on failure (D-024, D-031). Shared by
/// `T-Credit` and `T-Split` — both need a raw literal lowered at a currency's scale.
fn lower_amount(
    literal: &DecimalLiteral,
    scale: u32,
    currency_name: &str,
    diags: &mut Vec<Diagnostic>,
) -> Option<Amount> {
    match Amount::from_literal(&literal.text, scale) {
        Ok(amount) => Some(amount),
        Err(LiteralError::TooManyFractionDigits(frac_digits)) => {
            diags.push(Diagnostic::new(
                Code::TooManyFractionDigits,
                format!(
                    "amount has {frac_digits} fractional digits, but currency '{currency_name}' has scale {scale}"
                ),
                literal.span,
            ));
            None
        }
        Err(LiteralError::OutOfRange) => {
            diags.push(Diagnostic::new(
                Code::AmountOutOfRange,
                "this amount is too large to represent",
                literal.span,
            ));
            None
        }
    }
}

fn kind_mismatch(expected: BindingKind, span: Span) -> Diagnostic {
    match expected {
        BindingKind::Money => Diagnostic::new(
            Code::ExpectedMoney,
            "this is a conversion residue, not money — use 'absorb', not 'debit', to discharge it",
            span,
        ),
        BindingKind::Residue => Diagnostic::new(
            Code::ExpectedResidue,
            "this is money, not a conversion residue — use 'debit', not 'absorb', to discharge it",
            span,
        ),
    }
}

fn currency_mismatch(
    account: &AccountInfo,
    currencies: &[CurrencyInfo],
    found: CurrencyId,
    found_span: Span,
    use_span: Span,
) -> Diagnostic {
    let expected_name = &currencies[account.currency.0 as usize].name;
    let found_name = &currencies[found.0 as usize].name;
    Diagnostic::new(
        Code::CurrencyMismatch,
        format!(
            "account '{}' expects currency '{expected_name}', but this value is '{found_name}'",
            account.path
        ),
        use_span,
    )
    .with_secondary("this value's currency was established here", found_span)
}

fn rate_currency_mismatch(
    rate: &RateInfo,
    currencies: &[CurrencyInfo],
    found: CurrencyId,
    found_span: Span,
    convert_span: Span,
) -> Diagnostic {
    let expected_name = &currencies[rate.from.0 as usize].name;
    let found_name = &currencies[found.0 as usize].name;
    Diagnostic::new(
        Code::CurrencyMismatch,
        format!(
            "rate '{}' expects currency '{expected_name}', but this money is '{found_name}'",
            rate.name
        ),
        convert_span,
    )
    .with_secondary("this money's currency was established here", found_span)
}

fn merge_currency_mismatch(
    currencies: &[CurrencyInfo],
    a_currency: CurrencyId,
    a_span: Span,
    b_currency: CurrencyId,
    b_span: Span,
    merge_span: Span,
) -> Diagnostic {
    let a_name = &currencies[a_currency.0 as usize].name;
    let b_name = &currencies[b_currency.0 as usize].name;
    Diagnostic::new(
        Code::CurrencyMismatch,
        format!("merge's two values have different currencies: '{a_name}' and '{b_name}'"),
        merge_span,
    )
    .with_secondary("this value's currency was established here", a_span)
    .with_secondary("this value's currency was established here", b_span)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn etb() -> CurrencyInfo {
        CurrencyInfo { name: "ETB".to_string(), scale: 2 }
    }

    fn usd() -> CurrencyInfo {
        CurrencyInfo { name: "USD".to_string(), scale: 2 }
    }

    fn account(path: &str, currency: CurrencyId) -> AccountInfo {
        AccountInfo {
            path: path.to_string(),
            currency,
            kind: crate::resolve::AccountKind::Asset,
            normal: crate::parse::ast::NormalBalance::Debit,
        }
    }

    fn lit(text: &str) -> DecimalLiteral {
        DecimalLiteral { text: text.to_string(), span: Span::new(0, text.len() as u32) }
    }

    #[test]
    fn check_credit_lowers_the_amount_at_the_account_currency_scale() {
        let mut diags = Vec::new();
        let result = check_credit(CurrencyId(0), &etb(), &lit("45.00"), &mut diags);
        assert!(diags.is_empty());
        let (amount, currency) = result.unwrap();
        assert_eq!(amount, Amount::from_literal("45.00", 2).unwrap());
        assert_eq!(currency, CurrencyId(0));
    }

    #[test]
    fn check_credit_rejects_too_many_fraction_digits() {
        let mut diags = Vec::new();
        let result = check_credit(CurrencyId(0), &etb(), &lit("45.123"), &mut diags);
        assert!(result.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::TooManyFractionDigits);
    }

    #[test]
    fn check_account_currency_accepts_a_matching_currency() {
        let account = account("assets:cash", CurrencyId(0));
        let result =
            check_account_currency(&account, &[etb()], CurrencyId(0), Span::new(0, 1), Span::new(1, 2));
        assert!(result.is_ok());
    }

    #[test]
    fn check_account_currency_rejects_a_mismatched_currency() {
        let account = account("assets:cash", CurrencyId(0));
        let err = check_account_currency(
            &account,
            &[etb(), usd()],
            CurrencyId(1),
            Span::new(0, 1),
            Span::new(1, 2),
        )
        .unwrap_err();
        assert_eq!(err.code, Code::CurrencyMismatch);
        assert_eq!(err.secondary.len(), 1);
    }

    #[test]
    fn check_merge_accepts_matching_currencies_and_rejects_mismatched_ones() {
        let currencies = [etb(), usd()];
        assert!(check_merge(&currencies, CurrencyId(0), Span::new(0, 1), CurrencyId(0), Span::new(1, 2), Span::new(2, 3))
            .is_ok());
        let err = check_merge(
            &currencies,
            CurrencyId(0),
            Span::new(0, 1),
            CurrencyId(1),
            Span::new(1, 2),
            Span::new(2, 3),
        )
        .unwrap_err();
        assert_eq!(err.code, Code::CurrencyMismatch);
        assert_eq!(err.secondary.len(), 2);
    }

    #[test]
    fn check_split_ratio_rejects_both_weights_zero_but_accepts_any_other_pair() {
        assert!(check_split_ratio(0, 0, Span::new(0, 1)).is_err());
        assert!(check_split_ratio(1, 0, Span::new(0, 1)).is_ok());
        assert!(check_split_ratio(0, 1, Span::new(0, 1)).is_ok());
        assert!(check_split_ratio(1, 2, Span::new(0, 1)).is_ok());
    }
}
