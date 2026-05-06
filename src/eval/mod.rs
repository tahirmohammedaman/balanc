//! `typeck::TModule` -> `LedgerState`, tree-walking. `eval` never fabricates `Money`
//! and never re-checks linearity — by the time a `TModule` reaches here, `typeck` has
//! already guaranteed every binding is used exactly once. A `Var` that isn't in `env`
//! when evaluated would mean that guarantee didn't hold, i.e. a bug in `typeck`, not a
//! case for eval to handle gracefully — hence the `expect`, not a runtime check.

use std::collections::{BTreeMap, HashMap};

use crate::amount::Amount;
use crate::resolve::SymbolId;
use crate::typeck::{TModule, TMoneyExpr, TStmt};

/// Per-account running balance, keyed by the account path's rendered form
/// (`"assets:cash"`). Debits increase a balance, credits decrease it — the raw
/// double-entry convention with no normal-balance sign flip yet (that lands with
/// account declarations in Slice 5).
pub type LedgerState = BTreeMap<String, Amount>;

pub fn eval(module: &TModule) -> LedgerState {
    let mut ledger = LedgerState::new();
    for txn in &module.txns {
        let mut env: HashMap<SymbolId, Amount> = HashMap::new();
        for stmt in &txn.stmts {
            match stmt {
                TStmt::Let { symbol, value, .. } => {
                    let amount = eval_money_expr(value, &mut env, &mut ledger);
                    env.insert(*symbol, amount);
                }
                TStmt::Debit { account, value, .. } => {
                    let amount = eval_money_expr(value, &mut env, &mut ledger);
                    post(&mut ledger, account.to_string(), amount);
                }
            }
        }
    }
    ledger
}

/// Evaluating a `credit` is the moment money enters the system: it posts immediately
/// to its source account and hands back the amount to move onward. Evaluating a `Var`
/// moves the amount out of `env` — the runtime counterpart of Γ's `consume`.
fn eval_money_expr(
    expr: &TMoneyExpr,
    env: &mut HashMap<SymbolId, Amount>,
    ledger: &mut LedgerState,
) -> Amount {
    match expr {
        TMoneyExpr::Credit { account, amount } => {
            post(ledger, account.to_string(), -*amount);
            *amount
        }
        TMoneyExpr::Var { symbol } => env
            .remove(symbol)
            .expect("internal error: typeck guaranteed this binding is live and unconsumed"),
    }
}

fn post(ledger: &mut LedgerState, account: String, delta: Amount) {
    let balance = ledger.entry(account).or_insert(Amount::ZERO);
    *balance = *balance + delta;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::lex;
    use crate::parse::parse;
    use crate::resolve::resolve;
    use crate::typeck::typeck;

    fn eval_src(src: &str) -> LedgerState {
        let (tokens, _) = lex(src);
        let (module, _) = parse(&tokens.unwrap());
        let (resolved, _) = resolve(module.unwrap());
        let (typed, diags) = typeck(resolved.unwrap());
        assert!(diags.is_empty(), "typeck failed: {diags:?}");
        eval(&typed.unwrap())
    }

    #[test]
    fn let_bound_credit_flows_into_its_debit() {
        let ledger = eval_src(
            r#"txn "coffee" { let m = credit(assets:cash, 45.00); debit(expenses:coffee, m); }"#,
        );
        assert_eq!(ledger["expenses:coffee"].cents(), 4500);
        assert_eq!(ledger["assets:cash"].cents(), -4500);
    }

    #[test]
    fn inline_credit_posts_both_legs() {
        let ledger =
            eval_src(r#"txn "coffee" { debit(expenses:coffee, credit(assets:cash, 45.00)); }"#);
        assert_eq!(ledger["expenses:coffee"].cents(), 4500);
        assert_eq!(ledger["assets:cash"].cents(), -4500);
    }
}
