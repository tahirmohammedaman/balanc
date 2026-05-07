//! Inference rules for the balanc type system, written before the code that
//! implements them (ground rule). `Γ` is the linear context: the set of not-yet-
//! consumed `Money` bindings in scope, each tagged with the currency it was produced
//! at (Slice 2).
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
//!             Γ ⊢ body ⇒ Δ        Δ = ∅        debits(body) = credits(body)
//! (T-Txn)     ──────────────────────────────────────────────────────────────
//!             ⊢ txn { body } : Txn
//! ```
//!
//! `Δ = ∅` is invariant 1 (linear use); `Context::finish_txn` below is what checks it,
//! reporting every still-live binding as `E_DROPPED`. `Context::consume` is what
//! "consumes e" means operationally: remove it from Γ, or `E_REUSED` if it is already
//! gone.
//!
//! The balance premise is invariant 3. Through Slice 3 it is discharged *structurally*
//! rather than by an explicit sum: the grammar has no way to create a `Money` value
//! except `credit`, no way to destroy one except `debit`, and no arithmetic in
//! between — so `Δ = ∅` alone already forces every credited amount to reach exactly
//! one debit. An explicit computed check is deferred to Slice 4 (`split`/`merge`),
//! where that structural argument stops holding; `Code::Unbalanced` (in
//! `src/diag/codes.rs`) is reserved for it.
//!
//! The `acct : Account<C>` premise is now checked (Slice 2, D-022): every `credit` and
//! `debit` names a declared account, resolved to an `AccountId` by `resolve`, whose
//! declared `CurrencyId` `typeck` looks up. `Money<C>` is likewise now real: `Context`
//! tags each live binding with the `CurrencyId` it was produced at, so `T-Debit`'s
//! premise that `e`'s currency matches the target account's can actually be checked —
//! a mismatch is `E_CURRENCY_MISMATCH`, naming both the establishing site (where `e`'s
//! currency was fixed — the original `credit`) and the debit's account reference.

use std::collections::HashMap;

use crate::diag::{Code, Diagnostic};
use crate::resolve::{CurrencyId, SymbolId};
use crate::span::Span;

/// Types in the balanc type system. Only `Money` is linear and only it is tracked in
/// Γ. `Amount` (a literal argument), `Effect` (a `debit` statement's result), and
/// `Txn` (a whole transaction) exist here for the record — the grammar does not yet
/// let a program construct a value of `Effect` or `Txn` that would need checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    Money(CurrencyId),
    Amount,
    Effect,
    Txn,
}

/// A live binding tracks two spans that answer different questions: `binding_span` is
/// the `let`'s own binding site — what `E_DROPPED` points at if this is never
/// consumed — while `currency_span` is wherever this binding's currency was first
/// fixed (the original `credit`, forwarded through any number of `let`s) — what an
/// `E_CURRENCY_MISMATCH` cites as "this money's currency was established here". For a
/// binding whose value is a direct `credit`, these coincide only for the very first
/// `let`; forwarding through more `let`s keeps `currency_span` pinned to the original
/// `credit` while `binding_span` moves to each new name.
#[derive(Clone, Copy)]
struct Binding {
    binding_span: Span,
    currency_span: Span,
    currency: CurrencyId,
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

impl Context {
    /// Adds a fresh live binding. Called once per `let`, after its right-hand side has
    /// already been checked (and, if that side consumed something, after that
    /// consumption has already happened) — so a binding can never see itself.
    /// `binding_span` is the `let`'s own binding site; `currency_span` is where this
    /// value's currency was established (forwarded from the right-hand side).
    pub fn bind(&mut self, symbol: SymbolId, binding_span: Span, currency_span: Span, currency: CurrencyId) {
        self.live.insert(symbol, Binding { binding_span, currency_span, currency });
    }

    /// (T-Debit's "consumes e") Removes `symbol` from Γ, returning the currency and
    /// the span where it was established so the caller can check it against the
    /// account it's flowing into (or forward it, if consumed by another `let`).
    ///
    /// A `Var` naming a symbol that was never bound at all cannot reach here: the
    /// resolver already rejected that name as `E_UNBOUND_NAME` before typeck runs.
    pub fn consume(&mut self, symbol: SymbolId, use_span: Span) -> Result<(Span, CurrencyId), Diagnostic> {
        if let Some(binding) = self.live.remove(&symbol) {
            self.consumed.insert(symbol, use_span);
            Ok((binding.currency_span, binding.currency))
        } else {
            let first_span = *self
                .consumed
                .get(&symbol)
                .expect("internal error: consumed symbol has no recorded first-use span");
            Err(Diagnostic::new(
                Code::Reused,
                "this money value has already been consumed",
                use_span,
            )
            .with_secondary("first consumed here", first_span))
        }
    }

    /// (T-Txn's `Δ = ∅`) Everything still live at the end of the body was dropped.
    pub fn finish_txn(self) -> Vec<Diagnostic> {
        self.live
            .into_values()
            .map(|binding| {
                Diagnostic::new(
                    Code::Dropped,
                    "this money value is never consumed",
                    binding.binding_span,
                )
            })
            .collect()
    }
}
