//! Inference rules for the balanc type system, written before the code that
//! implements them (ground rule). `Γ` is the linear context: the set of not-yet-
//! consumed `Money` bindings in scope.
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
//! There is no `Account<C>` premise checked yet — accounts are declared starting
//! Slice 5 — and `Money` is not yet parameterized by currency (Slice 2), so both are
//! written into the rules above for the record but not separately encoded in `Ty`.

use std::collections::HashMap;

use crate::diag::{Code, Diagnostic};
use crate::resolve::SymbolId;
use crate::span::Span;

/// Types in the balanc type system. Only `Money` is linear and only it is tracked in
/// Γ. `Amount` (a literal argument), `Effect` (a `debit` statement's result), and
/// `Txn` (a whole transaction) exist here for the record — the grammar does not yet
/// let a program construct a value of any of them that would need checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    Money,
    Amount,
    Effect,
    Txn,
}

/// Γ, scoped to one transaction body (D-013: transactions are closed).
#[derive(Default)]
pub struct Context {
    /// Live bindings: symbol -> the span where it was bound. That span is what
    /// `E_DROPPED` points at if the binding is still here when the body ends.
    live: HashMap<SymbolId, Span>,
    /// Every symbol ever consumed, mapping to the span of that consumption. Kept
    /// after removal from `live` so a second consumption can name both use-sites.
    consumed: HashMap<SymbolId, Span>,
}

impl Context {
    /// Adds a fresh live binding. Called once per `let`, after its right-hand side has
    /// already been checked (and, if that side consumed something, after that
    /// consumption has already happened) — so a binding can never see itself.
    pub fn bind(&mut self, symbol: SymbolId, binding_span: Span) {
        self.live.insert(symbol, binding_span);
    }

    /// (T-Debit's "consumes e") Removes `symbol` from Γ.
    ///
    /// A `Var` naming a symbol that was never bound at all cannot reach here: the
    /// resolver already rejected that name as `E_UNBOUND_NAME` before typeck runs.
    pub fn consume(&mut self, symbol: SymbolId, use_span: Span) -> Result<(), Diagnostic> {
        if self.live.remove(&symbol).is_some() {
            self.consumed.insert(symbol, use_span);
            Ok(())
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
            .map(|binding_span| {
                Diagnostic::new(Code::Dropped, "this money value is never consumed", binding_span)
            })
            .collect()
    }
}
