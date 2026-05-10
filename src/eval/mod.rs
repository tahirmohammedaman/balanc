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
//! whatever produced it. `split`'s side condition (Slice 4, D-034 — the amount handed
//! to one half can't exceed what the input is actually worth) is the same story, one
//! level further: it's the first check `eval` can fail on its own, which is why this
//! stage's signature now matches every other one, `(Option<Output>, Vec<Diagnostic>)`,
//! instead of returning a bare `LedgerState` unconditionally.

use std::collections::{BTreeMap, HashMap};

use crate::amount::Amount;
use crate::diag::{Code, Diagnostic};
use crate::resolve::{AccountInfo, CurrencyInfo, RateInfo, SymbolId};
use crate::typeck::{TModule, TMoneyExpr, TStmt, TTxnDecl};

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

/// Evaluates every transaction independently (mirroring `typeck_txn`'s one-`Context`-
/// per-transaction shape): a `split` whose bound fails (D-034) stops *that*
/// transaction's remaining statements — its own already-live bindings and partial
/// ledger postings are abandoned, since nothing consumes an `env` that eval itself
/// gave up on mid-transaction — but later transactions still run, so a source file
/// with two independent errors reports both. Whether *any* diagnostic fired at all,
/// from any transaction, decides `Some`/`None` for the whole result, exactly like
/// every earlier stage — a partial `ledger` is never handed back to the caller.
pub fn eval(module: &TModule) -> (Option<LedgerState>, Vec<Diagnostic>) {
    let mut ledger = LedgerState::new();
    let mut diags = Vec::new();
    for txn in &module.txns {
        eval_txn(txn, &module.accounts, &module.currencies, &module.rates, &mut ledger, &mut diags);
    }
    if diags.is_empty() { (Some(ledger), diags) } else { (None, diags) }
}

fn eval_txn(
    txn: &TTxnDecl,
    accounts: &[AccountInfo],
    currencies: &[CurrencyInfo],
    rates: &[RateInfo],
    ledger: &mut LedgerState,
    diags: &mut Vec<Diagnostic>,
) {
    let mut env: HashMap<SymbolId, Value> = HashMap::new();
    for stmt in &txn.stmts {
        match stmt {
            TStmt::Let { symbol, value, .. } => {
                let amount = eval_money_expr(value, accounts, &mut env, ledger);
                env.insert(*symbol, Value::Money(amount));
            }
            TStmt::Convert { primary, residual, money, rate, span: _ } => {
                let amount = eval_money_expr(money, accounts, &mut env, ledger);
                let rate_info = &rates[rate.0 as usize];
                let (primary_amount, residual_amount, extra_scale) =
                    convert(amount, rate_info, currencies);
                env.insert(*primary, Value::Money(primary_amount));
                env.insert(*residual, Value::Residue { amount: residual_amount, extra_scale });
            }
            TStmt::Debit { account, value, .. } => {
                let amount = eval_money_expr(value, accounts, &mut env, ledger);
                post(ledger, &accounts[account.0 as usize].path, amount);
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
                post(ledger, &accounts[account.0 as usize].path, Amount::from_cents(whole_units));
            }
            TStmt::Split { a, b, money, currency, amount, amount_span, span: _ } => {
                let total = eval_money_expr(money, accounts, &mut env, ledger);
                if *amount > total {
                    let scale = currencies[currency.0 as usize].scale;
                    diags.push(Diagnostic::new(
                        Code::Unbalanced,
                        format!(
                            "cannot split off {} — only {} is available",
                            amount.format(scale),
                            total.format(scale)
                        ),
                        *amount_span,
                    ));
                    return;
                }
                env.insert(*a, Value::Money(*amount));
                env.insert(*b, Value::Money(total - *amount));
            }
            TStmt::SplitRatio { a, b, money, a_weight, b_weight, span: _ } => {
                let total = eval_money_expr(money, accounts, &mut env, ledger);
                let (a_amount, b_amount) = split_ratio(total, *a_weight, *b_weight);
                env.insert(*a, Value::Money(a_amount));
                env.insert(*b, Value::Money(b_amount));
            }
        }
    }
}

/// Evaluating a `credit` is the moment money enters the system: it posts immediately
/// to its source account and hands back the amount to move onward. Evaluating a `Var`
/// moves the amount out of `env` — the runtime counterpart of Γ's `consume`.
/// Evaluating a `merge` (T-Merge) evaluates both sides — in the same left-to-right
/// order `typeck` already checked them in, so a chain of `Var`s comes out of `env` in
/// the order the program named them — and adds the two amounts (D-031: this can
/// overflow `Amount::add`'s own panic guard, same as ordinary ledger accumulation
/// already could; deliberately left as-is, see D-031). Every `TMoneyExpr` position is
/// `Money`-typed by construction (`typeck` already rejected any `Residue` used here as
/// `E_EXPECTED_MONEY`), so every `env` lookup here always expects `Value::Money`.
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
        TMoneyExpr::Merge { a, b } => {
            let a = eval_money_expr(a, accounts, env, ledger);
            let b = eval_money_expr(b, accounts, env, ledger);
            a + b
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

/// T-SplitRatio's largest-remainder allocation (D-015, D-035): splits `total` in ratio
/// `a_weight : b_weight` as exactly as integer minor units allow, then gives the one
/// unit of rounding shortfall (if any) to whichever side's ideal share had the larger
/// fractional remainder — ties toward `a` (D-015's tie-break). For exactly two parts,
/// flooring both ideal shares can undershoot `total` by at most one minor unit: if
/// `a_floor + b_floor` already equalled `total` for every input, the two remainders
/// (`a_num mod total_weight` and `b_num mod total_weight`) would always be zero, which
/// they aren't whenever the split isn't perfectly even — that's exactly the case this
/// function exists to handle correctly rather than silently letting `b` absorb it
/// (D-015 explicitly rejected that as arbitrary).
fn split_ratio(total: Amount, a_weight: u32, b_weight: u32) -> (Amount, Amount) {
    let total_weight = a_weight as i128 + b_weight as i128;
    let t = total.cents() as i128;
    let a_num = t * a_weight as i128;
    let b_num = t * b_weight as i128;
    let a_floor = a_num / total_weight;
    let b_floor = b_num / total_weight;
    let a_rem = a_num % total_weight;
    let b_rem = b_num % total_weight;
    let shortfall = t - a_floor - b_floor;
    let (a_final, b_final) =
        if a_rem >= b_rem { (a_floor + shortfall, b_floor) } else { (a_floor, b_floor + shortfall) };
    (Amount::from_cents(a_final as i64), Amount::from_cents(b_final as i64))
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
        let (ledger, diags) = eval(&typed.unwrap());
        assert!(diags.is_empty(), "eval failed: {diags:?}");
        ledger.unwrap()
    }

    fn eval_src_diags(src: &str) -> Vec<Diagnostic> {
        let src = format!("{PRELUDE} {src}");
        let (tokens, _) = lex(&src);
        let (module, _) = parse(&tokens.unwrap());
        let (resolved, _) = resolve(module.unwrap());
        let (typed, diags) = typeck(resolved.unwrap());
        assert!(diags.is_empty(), "typeck failed: {diags:?}");
        let (ledger, diags) = eval(&typed.unwrap());
        assert!(ledger.is_none());
        diags
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

    #[test]
    fn split_divides_a_value_into_two_parts() {
        let ledger = eval_src(
            r#"txn "t" {
                let m = credit(assets:cash, 45.00);
                let (a, b) = split(m, 20.00);
                debit(expenses:coffee, a);
                debit(expenses:coffee, b);
            }"#,
        );
        assert_eq!(ledger["expenses:coffee"].cents(), 4500);
        assert_eq!(ledger["assets:cash"].cents(), -4500);
    }

    #[test]
    fn split_exceeding_the_input_reports_e_unbalanced() {
        let diags = eval_src_diags(
            r#"txn "t" {
                let m = credit(assets:cash, 45.00);
                let (a, b) = split(m, 100.00);
                debit(expenses:coffee, a);
                debit(expenses:coffee, b);
            }"#,
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::Unbalanced);
    }

    #[test]
    fn split_ratio_uses_largest_remainder_allocation() {
        // 100.01 split 1:2 -> ideal 3333.67:6667.33 in hundredths of a cent; largest
        // remainder gives the odd cent to whichever side's ideal share was higher.
        let ledger = eval_src(
            r#"txn "t" {
                let m = credit(assets:cash, 100.01);
                let (a, b) = split_ratio(m, 1, 2);
                debit(expenses:coffee, a);
                debit(expenses:coffee, b);
            }"#,
        );
        assert_eq!(ledger["expenses:coffee"].cents(), 10001);
        assert_eq!(ledger["assets:cash"].cents(), -10001);
    }

    #[test]
    fn merge_combines_two_values() {
        let ledger = eval_src(
            r#"txn "t" {
                let a = credit(assets:cash, 20.00);
                let b = credit(assets:cash, 25.00);
                debit(expenses:coffee, merge(a, b));
            }"#,
        );
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
        let (ledger, diags) = eval(&typed.unwrap());
        assert!(diags.is_empty(), "eval failed: {diags:?}");
        ledger.unwrap()
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

    // Property test (PLAN.md's Slice 4 proof-test): for a spread of amount/split-ratio
    // pairs, `split` parts always sum back to the input and `split_ratio`'s two parts
    // always sum back to the input too, regardless of how unevenly the ratio divides
    // it. A small deterministic xorshift PRNG stands in for a property-testing crate
    // (none is vendored in this project).
    #[test]
    fn split_and_split_ratio_conserve_the_total_over_random_inputs() {
        struct Rng(u64);
        impl Rng {
            fn next(&mut self) -> u64 {
                let mut x = self.0;
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                self.0 = x;
                x
            }
        }
        let mut rng = Rng(0xC0FFEE);
        for _ in 0..10_000 {
            let total_cents = (rng.next() % 10_000_000) as i64;
            let total = Amount::from_cents(total_cents);

            let split_point = Amount::from_cents((rng.next() % (total_cents as u64 + 1)) as i64);
            let (a, b) = (split_point, total - split_point);
            assert_eq!(a + b, total);

            let a_weight = (rng.next() % 100) as u32;
            let b_weight = (rng.next() % 100) as u32;
            if a_weight == 0 && b_weight == 0 {
                continue;
            }
            let (ra, rb) = split_ratio(total, a_weight, b_weight);
            assert_eq!(ra + rb, total, "split_ratio({total_cents}, {a_weight}, {b_weight})");
            assert!(!ra.is_negative() && !rb.is_negative());
        }
    }
}
