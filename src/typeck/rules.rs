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

use crate::diag::{Code, Diagnostic};
use crate::resolve::{CurrencyId, SymbolId};
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
