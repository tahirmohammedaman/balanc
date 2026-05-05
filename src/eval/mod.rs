//! `ast::Module` -> `LedgerState`, tree-walking. `eval` never fabricates `Money` and
//! never re-checks linearity or balance — those are `check`'s (later `typeck`'s) job;
//! by the time a `Module` reaches here it is assumed valid.

use std::collections::BTreeMap;

use crate::amount::Amount;
use crate::parse::ast::{LegKind, Module};

/// Per-account running balance, keyed by the account path's rendered form
/// (`"assets:cash"`). Debits increase a balance, credits decrease it — this is the
/// raw double-entry convention with no normal-balance sign flip yet (that lands with
/// account declarations in Slice 5).
pub type LedgerState = BTreeMap<String, Amount>;

pub fn eval(module: &Module) -> LedgerState {
    let mut ledger = LedgerState::new();
    for txn in &module.txns {
        for leg in &txn.legs {
            let balance = ledger.entry(leg.account.to_string()).or_insert(Amount::ZERO);
            *balance = match leg.kind {
                LegKind::Debit => *balance + leg.amount,
                LegKind::Credit => *balance - leg.amount,
            };
        }
    }
    ledger
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::lex;
    use crate::parse::parse;

    #[test]
    fn two_leg_transaction_produces_offsetting_balances() {
        let (tokens, _) = lex(
            r#"txn "coffee" { debit(expenses:coffee, 45.00); credit(assets:cash, 45.00); }"#,
        );
        let (module, _) = parse(&tokens.unwrap());
        let ledger = eval(&module.unwrap());
        assert_eq!(ledger["expenses:coffee"].cents(), 4500);
        assert_eq!(ledger["assets:cash"].cents(), -4500);
    }
}
