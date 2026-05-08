//! `typeck::TModule` -> `LedgerState`, tree-walking. `eval` never fabricates a `Money`
//! or `Residue` value and never re-checks linearity — by the time a `TModule` reaches
//! here, `typeck` has already guaranteed every binding is used exactly once, as the
//! right kind. A binding that isn't in `env` when evaluated, or is the wrong `Value`
//! variant, would mean that guarantee didn't hold, i.e. a bug in `typeck`, not a case
//! for eval to handle gracefully — hence the `expect`s, not runtime checks.
//!
//! `convert`'s arithmetic (Slice 3, D-026) lives here, not in `typeck`: unlike a
//! `credit`'s literal amount (known at typeck time), the money flowing into `convert`
//! is often a `Var` whose concrete amount isn't known until this stage evaluates
//! whatever produced it.

use std::collections::{BTreeMap, HashMap};

use crate::amount::Amount;
use crate::resolve::{AccountInfo, CurrencyInfo, RateInfo, SymbolId};
use crate::typeck::{TModule, TMoneyExpr, TStmt};

/// Per-account running balance, keyed by the account path's rendered form
/// (`"assets:cash"`) — every account has exactly one currency (its declaration), so
/// the path alone is enough to key by; `render` looks the currency back up via
/// `TModule::accounts` when it needs it. Debits (and `absorb`s) increase a balance,
/// credits decrease it — the raw double-entry convention with no normal-balance sign
/// flip yet (that lands with account declarations in Slice 5).
pub type LedgerState = BTreeMap<String, Amount>;

/// A binding's runtime value: an ordinary `Money` amount, or a `convert`-produced
/// `Residue` (D-026) — the fractional leftover from rounding, tracked as an integer at
/// `extra_scale` digits *beyond* its currency's nominal scale, since a single residue
/// is by construction smaller than one whole minor unit.
enum Value {
    Money(Amount),
    Residue { amount: i64, extra_scale: u32 },
}

pub fn eval(module: &TModule) -> LedgerState {
    let mut ledger = LedgerState::new();
    for txn in &module.txns {
        let mut env: HashMap<SymbolId, Value> = HashMap::new();
        for stmt in &txn.stmts {
            match stmt {
                TStmt::Let { symbol, value, .. } => {
                    let amount = eval_money_expr(value, &module.accounts, &mut env, &mut ledger);
                    env.insert(*symbol, Value::Money(amount));
                }
                TStmt::Convert { primary, residual, money, rate, span: _ } => {
                    let amount = eval_money_expr(money, &module.accounts, &mut env, &mut ledger);
                    let rate_info = &module.rates[rate.0 as usize];
                    let (primary_amount, residual_amount, extra_scale) =
                        convert(amount, rate_info, &module.currencies);
                    env.insert(*primary, Value::Money(primary_amount));
                    env.insert(
                        *residual,
                        Value::Residue { amount: residual_amount, extra_scale },
                    );
                }
                TStmt::Debit { account, value, .. } => {
                    let amount = eval_money_expr(value, &module.accounts, &mut env, &mut ledger);
                    post(&mut ledger, &module.accounts[account.0 as usize].path, amount);
                }
                TStmt::Absorb { residual, account, .. } => {
                    let (amount, extra_scale) = match env
                        .remove(residual)
                        .expect("internal error: typeck guaranteed this residue is live")
                    {
                        Value::Residue { amount, extra_scale } => (amount, extra_scale),
                        Value::Money(_) => {
                            panic!("internal error: typeck guaranteed absorb's argument is a residue")
                        }
                    };
                    let whole_units = amount / 10i64.pow(extra_scale);
                    post(&mut ledger, &module.accounts[account.0 as usize].path, Amount::from_cents(whole_units));
                }
            }
        }
    }
    ledger
}

/// Evaluating a `credit` is the moment money enters the system: it posts immediately
/// to its source account and hands back the amount to move onward. Evaluating a `Var`
/// moves the amount out of `env` — the runtime counterpart of Γ's `consume`. Every
/// `TMoneyExpr` position is `Money`-typed by construction (`typeck` already rejected
/// any `Residue` used here as `E_EXPECTED_MONEY`), so the `env` lookup always expects
/// `Value::Money`.
fn eval_money_expr(
    expr: &TMoneyExpr,
    accounts: &[AccountInfo],
    env: &mut HashMap<SymbolId, Value>,
    ledger: &mut LedgerState,
) -> Amount {
    match expr {
        TMoneyExpr::Credit { account, amount } => {
            post(ledger, &accounts[account.0 as usize].path, -*amount);
            *amount
        }
        TMoneyExpr::Var { symbol } => {
            match env.remove(symbol).expect("internal error: typeck guaranteed this binding is live") {
                Value::Money(amount) => amount,
                Value::Residue { .. } => {
                    panic!("internal error: typeck guaranteed this binding is money, not a residue")
                }
            }
        }
    }
}

/// The conversion math (T-Convert, D-026): computes the exact real-valued result of
/// applying `rate` to `amount` (`amount`'s currency is `rate.from`, checked by
/// `typeck`) as a single exact fraction, then splits it into a `primary` `Money`
/// value — the floor, in `rate.to`'s nominal minor units (D-027: floor is the only
/// conservation-safe choice) — and the exact `residual` remainder, which is smaller
/// than one `rate.to` minor unit by construction and is expressed as an integer at
/// `extra_scale = amount's currency's scale + rate.scale` digits of precision beyond
/// `rate.to`'s nominal scale — exact, because the shared denominator of the whole
/// computation is a power of 10 (see D-026's rationale for why this is free here but
/// not the other way around).
fn convert(amount: Amount, rate: &RateInfo, currencies: &[CurrencyInfo]) -> (Amount, i64, u32) {
    let from_scale = currencies[rate.from.0 as usize].scale;
    let to_scale = currencies[rate.to.0 as usize].scale;
    let extra_scale = from_scale + rate.scale;

    let numerator = (amount.cents() as i128) * (rate.numerator as i128) * 10i128.pow(to_scale);
    let denominator = 10i128.pow(extra_scale);

    let primary_minor_units = (numerator / denominator) as i64;
    let residual = (numerator % denominator) as i64;

    (Amount::from_cents(primary_minor_units), residual, extra_scale)
}

fn post(ledger: &mut LedgerState, account: &str, delta: Amount) {
    let balance = ledger.entry(account.to_string()).or_insert(Amount::ZERO);
    *balance = *balance + delta;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::lex;
    use crate::parse::parse;
    use crate::resolve::resolve;
    use crate::typeck::typeck;

    const PRELUDE: &str = r#"
        currency ETB { scale = 2 }
        account assets:cash { currency = ETB }
        account expenses:coffee { currency = ETB }
    "#;

    fn eval_src(src: &str) -> LedgerState {
        let src = format!("{PRELUDE} {src}");
        let (tokens, _) = lex(&src);
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

    const FX_PRELUDE: &str = r#"
        currency USD { scale = 2 }
        currency ETB { scale = 2 }
        account assets:usd_cash { currency = USD }
        account assets:etb_cash { currency = ETB }
        account income:fx_rounding { currency = ETB }
        rate usd_etb from USD to ETB = 57.20 round down;
    "#;

    fn fx_eval_src(src: &str) -> LedgerState {
        let src = format!("{FX_PRELUDE} {src}");
        let (tokens, _) = lex(&src);
        let (module, _) = parse(&tokens.unwrap());
        let (resolved, _) = resolve(module.unwrap());
        let (typed, diags) = typeck(resolved.unwrap());
        assert!(diags.is_empty(), "typeck failed: {diags:?}");
        eval(&typed.unwrap())
    }

    #[test]
    fn convert_computes_the_exact_rate_and_absorb_posts_the_whole_unit_portion() {
        // 100.00 USD * 57.20 = 5720.00 ETB exactly -- divides evenly, residue is 0.
        let ledger = fx_eval_src(
            r#"txn "fx" {
                let m = credit(assets:usd_cash, 100.00);
                let (m2, r) = convert(m, usd_etb);
                debit(assets:etb_cash, m2);
                absorb(r, income:fx_rounding);
            }"#,
        );
        assert_eq!(ledger["assets:usd_cash"].cents(), -10000);
        assert_eq!(ledger["assets:etb_cash"].cents(), 572000);
        // A single conversion's residue is always < 1 minor unit (D-028); absorbing it
        // alone posts zero, evenly-dividing rate or not.
        assert_eq!(ledger["income:fx_rounding"].cents(), 0);
    }

    #[test]
    fn convert_with_a_non_dividing_rate_still_absorbs_to_zero_for_one_conversion() {
        let ledger = fx_eval_src(
            r#"txn "fx" {
                let m = credit(assets:usd_cash, 100.01);
                let (m2, r) = convert(m, usd_etb);
                debit(assets:etb_cash, m2);
                absorb(r, income:fx_rounding);
            }"#,
        );
        // 100.01 * 57.20 = 5720.572 ETB exactly -> floors to 5720.57, losing 0.002.
        assert_eq!(ledger["assets:etb_cash"].cents(), 572057);
        assert_eq!(ledger["income:fx_rounding"].cents(), 0);
    }
}
