//! `resolve::ResolvedModule` -> `TModule` + `Diagnostic`s. The traversal driver: walks
//! each transaction body left to right, threading Γ (`rules::Context`) through `let`,
//! `convert`, `debit`, and `absorb` statements per the rules documented in `rules.rs`.

pub mod rules;

use crate::amount::{Amount, LiteralError};
use crate::diag::{Code, Diagnostic};
use crate::parse::ast::DecimalLiteral;
use crate::resolve::{
    AccountId, AccountInfo, CurrencyId, CurrencyInfo, RateId, RateInfo, ResolvedModule,
    ResolvedMoneyExpr, ResolvedStmt, ResolvedTxn, SymbolId,
};
use crate::span::Span;
use rules::{BindingKind, ConsumeResult, Context};
pub use rules::Ty;

pub struct TModule {
    pub currencies: Vec<CurrencyInfo>,
    pub accounts: Vec<AccountInfo>,
    pub rates: Vec<RateInfo>,
    pub txns: Vec<TTxnDecl>,
}

pub struct TTxnDecl {
    pub name: String,
    pub stmts: Vec<TStmt>,
    pub span: Span,
}

pub enum TStmt {
    Let { symbol: SymbolId, value: TMoneyExpr, span: Span },
    /// `let (primary, residual) = convert(money, rate);` (D-026). The actual
    /// arithmetic happens in `eval`, not here: `money`'s concrete amount generally
    /// isn't known until runtime (it may be a `Var` sourced from an earlier `credit`
    /// evaluated elsewhere), exactly like `debit`'s value already works.
    Convert { primary: SymbolId, residual: SymbolId, money: TMoneyExpr, rate: RateId, span: Span },
    Debit { account: AccountId, value: TMoneyExpr, span: Span },
    /// `absorb(residual, account);` (D-028).
    Absorb { residual: SymbolId, account: AccountId, span: Span },
    /// `let (a, b) = split(money, amount);` (D-034). `amount` is already lowered (it
    /// only needed `money`'s currency's scale, known once `money` typechecks) but
    /// whether it actually fits inside `money`'s runtime value can't be checked here
    /// — that's `eval`'s job, `amount_span` is what its diagnostic points at.
    Split {
        a: SymbolId,
        b: SymbolId,
        money: TMoneyExpr,
        currency: CurrencyId,
        amount: Amount,
        amount_span: Span,
        span: Span,
    },
    /// `let (a, b) = split_ratio(money, a_weight, b_weight);` (D-015/D-035). Always
    /// exact (T-SplitRatio) — no runtime side condition, unlike `Split`.
    SplitRatio { a: SymbolId, b: SymbolId, money: TMoneyExpr, a_weight: u32, b_weight: u32, span: Span },
}

pub enum TMoneyExpr {
    Credit { account: AccountId, amount: Amount },
    Var { symbol: SymbolId },
    /// `merge(a, b)` (T-Merge).
    Merge { a: Box<TMoneyExpr>, b: Box<TMoneyExpr> },
}

pub fn typeck(module: ResolvedModule) -> (Option<TModule>, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let txns = module
        .txns
        .into_iter()
        .map(|txn| typeck_txn(txn, &module.accounts, &module.currencies, &module.rates, &mut diags))
        .collect();
    if diags.is_empty() {
        let module =
            TModule { currencies: module.currencies, accounts: module.accounts, rates: module.rates, txns };
        (Some(module), diags)
    } else {
        (None, diags)
    }
}

fn typeck_txn(
    txn: ResolvedTxn,
    accounts: &[AccountInfo],
    currencies: &[CurrencyInfo],
    rates: &[RateInfo],
    diags: &mut Vec<Diagnostic>,
) -> TTxnDecl {
    let mut ctx = Context::default();
    let stmts = txn
        .stmts
        .into_iter()
        .filter_map(|stmt| match stmt {
            ResolvedStmt::Let { symbol, name_span, value, span } => {
                let (value, currency, currency_span) =
                    typeck_money_expr(value, accounts, currencies, &mut ctx, diags)?;
                ctx.bind(symbol, name_span, currency_span, currency, BindingKind::Money);
                Some(TStmt::Let { symbol, value, span })
            }
            ResolvedStmt::Convert { primary, primary_span, residual, residual_span, money, rate, span } => {
                let (money, money_currency, currency_span) =
                    typeck_money_expr(money, accounts, currencies, &mut ctx, diags)?;
                let rate_info = &rates[rate.0 as usize];
                if money_currency != rate_info.from {
                    diags.push(rate_currency_mismatch(
                        rate_info,
                        currencies,
                        money_currency,
                        currency_span,
                        span,
                    ));
                    return None;
                }
                ctx.bind(primary, primary_span, span, rate_info.to, BindingKind::Money);
                ctx.bind(residual, residual_span, span, rate_info.to, BindingKind::Residue);
                Some(TStmt::Convert { primary, residual, money, rate, span })
            }
            ResolvedStmt::Debit { account, value, span } => {
                let (value, value_currency, currency_span) =
                    typeck_money_expr(value, accounts, currencies, &mut ctx, diags)?;
                let account_currency = accounts[account.0 as usize].currency;
                if value_currency != account_currency {
                    diags.push(currency_mismatch(
                        &accounts[account.0 as usize],
                        currencies,
                        value_currency,
                        currency_span,
                        span,
                    ));
                    return None;
                }
                Some(TStmt::Debit { account, value, span })
            }
            ResolvedStmt::Absorb { residue, residue_span, account, span } => {
                let (currency_span, residue_currency) =
                    consume_checked(residue, residue_span, BindingKind::Residue, &mut ctx, diags)?;
                let account_currency = accounts[account.0 as usize].currency;
                if residue_currency != account_currency {
                    diags.push(currency_mismatch(
                        &accounts[account.0 as usize],
                        currencies,
                        residue_currency,
                        currency_span,
                        span,
                    ));
                    return None;
                }
                Some(TStmt::Absorb { residual: residue, account, span })
            }
            ResolvedStmt::Split { a, a_span, b, b_span, money, amount, span } => {
                let (money, money_currency, currency_span) =
                    typeck_money_expr(money, accounts, currencies, &mut ctx, diags)?;
                let scale = currencies[money_currency.0 as usize].scale;
                let lowered_amount = lower_amount(
                    &amount,
                    scale,
                    &currencies[money_currency.0 as usize].name,
                    diags,
                )?;
                ctx.bind(a, a_span, currency_span, money_currency, BindingKind::Money);
                ctx.bind(b, b_span, currency_span, money_currency, BindingKind::Money);
                Some(TStmt::Split {
                    a,
                    b,
                    money,
                    currency: money_currency,
                    amount: lowered_amount,
                    amount_span: amount.span,
                    span,
                })
            }
            ResolvedStmt::SplitRatio {
                a,
                a_span,
                b,
                b_span,
                money,
                a_weight,
                a_weight_span,
                b_weight,
                b_weight_span,
                span,
            } => {
                let (money, money_currency, currency_span) =
                    typeck_money_expr(money, accounts, currencies, &mut ctx, diags)?;
                if a_weight == 0 && b_weight == 0 {
                    diags.push(Diagnostic::new(
                        Code::ZeroRatio,
                        "split_ratio's weights are both zero, which doesn't determine an allocation",
                        a_weight_span.to(b_weight_span),
                    ));
                    return None;
                }
                ctx.bind(a, a_span, currency_span, money_currency, BindingKind::Money);
                ctx.bind(b, b_span, currency_span, money_currency, BindingKind::Money);
                Some(TStmt::SplitRatio { a, b, money, a_weight, b_weight, span })
            }
        })
        .collect();
    diags.extend(ctx.finish_txn());
    TTxnDecl { name: txn.name, stmts, span: txn.span }
}

/// Consumes `symbol` from Γ and checks it was the expected kind (`Money` or
/// `Residue`), reporting `E_EXPECTED_MONEY`/`E_EXPECTED_RESIDUE` if not (D-026). Shared
/// by every consuming site (`debit`'s/`convert`'s money argument via
/// `typeck_money_expr`, and `absorb`'s residue argument directly) so there is one
/// place that turns "wrong kind of value" into a diagnostic.
fn consume_checked(
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

/// Type-checks a `MoneyExpr`, returning its lowered IR, its currency (T-Credit's `C`,
/// or the currency the consumed binding carried), and the span where that currency was
/// established — the original `credit`'s span, forwarded through any `let`s.
fn typeck_money_expr(
    expr: ResolvedMoneyExpr,
    accounts: &[AccountInfo],
    currencies: &[CurrencyInfo],
    ctx: &mut Context,
    diags: &mut Vec<Diagnostic>,
) -> Option<(TMoneyExpr, CurrencyId, Span)> {
    match expr {
        ResolvedMoneyExpr::Credit { account, amount, span } => {
            let info = &accounts[account.0 as usize];
            let currency = info.currency;
            let scale = currencies[currency.0 as usize].scale;
            let amount =
                lower_amount(&amount, scale, &currencies[currency.0 as usize].name, diags)?;
            Some((TMoneyExpr::Credit { account, amount }, currency, span))
        }
        ResolvedMoneyExpr::Var { symbol, span } => {
            let (currency_span, currency) =
                consume_checked(symbol, span, BindingKind::Money, ctx, diags)?;
            Some((TMoneyExpr::Var { symbol }, currency, currency_span))
        }
        ResolvedMoneyExpr::Merge { a, b, span } => {
            // Typecheck both sides unconditionally (not `a?` then `b`), matching
            // `resolve`'s handling of the same node, so a problem on one side doesn't
            // hide a diagnostic on the other.
            let a_result = typeck_money_expr(*a, accounts, currencies, ctx, diags);
            let b_result = typeck_money_expr(*b, accounts, currencies, ctx, diags);
            let (a_expr, a_currency, a_span) = a_result?;
            let (b_expr, b_currency, b_span) = b_result?;
            if a_currency != b_currency {
                diags.push(merge_currency_mismatch(
                    currencies, a_currency, a_span, b_currency, b_span, span,
                ));
                return None;
            }
            Some((TMoneyExpr::Merge { a: Box::new(a_expr), b: Box::new(b_expr) }, a_currency, span))
        }
    }
}

/// Lowers a decimal literal into an `Amount` at `scale`, reporting
/// `TooManyFractionDigits`/`AmountOutOfRange` on failure (D-024, D-031). Shared by
/// `credit`'s amount (`typeck_money_expr`) and `split`'s (T-Split) — both are a raw
/// literal that only needs a currency's scale to lower, known once the money value
/// it's paired with has typechecked.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Code;
    use crate::lex::lex;
    use crate::parse::parse;
    use crate::resolve::resolve;

    const PRELUDE: &str = r#"
        currency ETB { scale = 2 }
        currency USD { scale = 2 }
        account assets:cash { currency = ETB }
        account assets:usd_cash { currency = USD }
        account expenses:coffee { currency = ETB }
        account expenses:a { currency = ETB }
        account expenses:b { currency = ETB }
    "#;

    const FX_PRELUDE: &str = r#"
        currency USD { scale = 2 }
        currency ETB { scale = 2 }
        account assets:usd_cash { currency = USD }
        account assets:etb_cash { currency = ETB }
        account income:fx_rounding { currency = ETB }
        rate usd_etb from USD to ETB = 57.20 round down;
    "#;

    fn typeck_src(src: &str) -> (Option<TModule>, Vec<Diagnostic>) {
        typeck_src_no_prelude(&format!("{PRELUDE} {src}"))
    }

    fn typeck_src_no_prelude(src: &str) -> (Option<TModule>, Vec<Diagnostic>) {
        let (tokens, _) = lex(src);
        let (module, _) = parse(&tokens.unwrap());
        let (resolved, diags) = resolve(module.unwrap());
        assert!(diags.is_empty(), "resolve failed: {diags:?}");
        typeck(resolved.unwrap())
    }

    #[test]
    fn well_formed_transaction_typechecks() {
        let (module, diags) = typeck_src(
            r#"txn "coffee" { let m = credit(assets:cash, 45.00); debit(expenses:coffee, m); }"#,
        );
        assert!(diags.is_empty());
        assert!(module.is_some());
    }

    #[test]
    fn dropped_value_is_reported_at_its_binding() {
        let (module, diags) =
            typeck_src(r#"txn "coffee" { let m = credit(assets:cash, 45.00); }"#);
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::Dropped);
    }

    #[test]
    fn reused_value_is_reported_with_both_spans() {
        let (module, diags) = typeck_src(
            r#"txn "coffee" {
                let m = credit(assets:cash, 45.00);
                debit(expenses:a, m);
                debit(expenses:b, m);
            }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::Reused);
        assert_eq!(diags[0].secondary.len(), 1);
        assert_eq!(diags[0].secondary[0].0, "first consumed here");
    }

    #[test]
    fn inline_credit_in_a_debit_needs_no_binding() {
        let (module, diags) =
            typeck_src(r#"txn "coffee" { debit(expenses:coffee, credit(assets:cash, 45.00)); }"#);
        assert!(diags.is_empty());
        assert!(module.is_some());
    }

    #[test]
    fn currency_mismatch_is_reported_with_both_spans() {
        let (module, diags) = typeck_src(
            r#"txn "t" {
                let m = credit(assets:usd_cash, 45.00);
                debit(expenses:coffee, m);
            }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::CurrencyMismatch);
        assert_eq!(diags[0].secondary.len(), 1);
        assert_eq!(diags[0].secondary[0].0, "this value's currency was established here");
    }

    #[test]
    fn too_many_fraction_digits_for_currency_scale_is_reported() {
        let (module, diags) = typeck_src_no_prelude(
            r#"currency JPY { scale = 0 }
               account assets:cash { currency = JPY }
               account expenses:coffee { currency = JPY }
               txn "t" { debit(expenses:coffee, credit(assets:cash, 45.50)); }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::TooManyFractionDigits);
    }

    #[test]
    fn amount_too_large_to_represent_is_reported() {
        // Fix checkpoint B: fuzzing found this panicking instead of producing a
        // diagnostic (a lexer-valid literal's magnitude can still overflow `i64`).
        let (module, diags) = typeck_src_no_prelude(&format!(
            r#"currency JPY {{ scale = 0 }}
               account assets:cash {{ currency = JPY }}
               account expenses:coffee {{ currency = JPY }}
               txn "t" {{ debit(expenses:coffee, credit(assets:cash, {})); }}"#,
            "9".repeat(30)
        ));
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::AmountOutOfRange);
    }

    fn fx_src(src: &str) -> (Option<TModule>, Vec<Diagnostic>) {
        typeck_src_no_prelude(&format!("{FX_PRELUDE} {src}"))
    }

    #[test]
    fn well_formed_convert_and_absorb_typechecks() {
        let (module, diags) = fx_src(
            r#"txn "fx" {
                let m = credit(assets:usd_cash, 100.00);
                let (m2, r) = convert(m, usd_etb);
                debit(assets:etb_cash, m2);
                absorb(r, income:fx_rounding);
            }"#,
        );
        assert!(diags.is_empty(), "{diags:?}");
        assert!(module.is_some());
    }

    #[test]
    fn dropped_residue_is_reported() {
        let (module, diags) = fx_src(
            r#"txn "fx" {
                let m = credit(assets:usd_cash, 100.00);
                let (m2, r) = convert(m, usd_etb);
                debit(assets:etb_cash, m2);
            }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::Dropped);
    }

    #[test]
    fn convert_with_wrong_currency_money_is_reported() {
        let (module, diags) = fx_src(
            r#"txn "fx" {
                let m = credit(assets:etb_cash, 100.00);
                let (m2, r) = convert(m, usd_etb);
                debit(assets:etb_cash, m2);
                absorb(r, income:fx_rounding);
            }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::CurrencyMismatch);
    }

    #[test]
    fn debiting_a_residue_is_reported() {
        let (module, diags) = fx_src(
            r#"txn "fx" {
                let m = credit(assets:usd_cash, 100.00);
                let (m2, r) = convert(m, usd_etb);
                debit(assets:etb_cash, m2);
                debit(income:fx_rounding, r);
            }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::ExpectedMoney);
    }

    #[test]
    fn absorbing_money_is_reported() {
        let (module, diags) = fx_src(
            r#"txn "fx" {
                let m = credit(assets:usd_cash, 100.00);
                let (m2, r) = convert(m, usd_etb);
                absorb(m2, income:fx_rounding);
                absorb(r, income:fx_rounding);
            }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::ExpectedResidue);
    }

    #[test]
    fn well_formed_split_typechecks() {
        let (module, diags) = typeck_src(
            r#"txn "t" {
                let m = credit(assets:cash, 45.00);
                let (a, b) = split(m, 20.00);
                debit(expenses:a, a);
                debit(expenses:b, b);
            }"#,
        );
        assert!(diags.is_empty(), "{diags:?}");
        assert!(module.is_some());
    }

    #[test]
    fn dropping_one_half_of_a_split_is_reported() {
        let (module, diags) = typeck_src(
            r#"txn "t" {
                let m = credit(assets:cash, 45.00);
                let (a, b) = split(m, 20.00);
                debit(expenses:a, a);
            }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::Dropped);
    }

    #[test]
    fn well_formed_split_ratio_typechecks() {
        let (module, diags) = typeck_src(
            r#"txn "t" {
                let m = credit(assets:cash, 45.00);
                let (a, b) = split_ratio(m, 1, 2);
                debit(expenses:a, a);
                debit(expenses:b, b);
            }"#,
        );
        assert!(diags.is_empty(), "{diags:?}");
        assert!(module.is_some());
    }

    #[test]
    fn split_ratio_with_both_weights_zero_is_reported() {
        let (module, diags) = typeck_src(
            r#"txn "t" {
                let m = credit(assets:cash, 45.00);
                let (a, b) = split_ratio(m, 0, 0);
                debit(expenses:a, a);
                debit(expenses:b, b);
            }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::ZeroRatio);
    }

    #[test]
    fn well_formed_merge_typechecks() {
        let (module, diags) = typeck_src(
            r#"txn "t" {
                let a = credit(assets:cash, 20.00);
                let b = credit(assets:cash, 25.00);
                debit(expenses:coffee, merge(a, b));
            }"#,
        );
        assert!(diags.is_empty(), "{diags:?}");
        assert!(module.is_some());
    }

    #[test]
    fn merging_a_value_with_itself_is_reported_as_reused() {
        let (module, diags) = typeck_src(
            r#"txn "t" {
                let m = credit(assets:cash, 20.00);
                debit(expenses:coffee, merge(m, m));
            }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::Reused);
    }

    #[test]
    fn merging_different_currencies_is_reported() {
        let (module, diags) = typeck_src(
            r#"txn "t" {
                let a = credit(assets:cash, 20.00);
                let b = credit(assets:usd_cash, 20.00);
                debit(expenses:coffee, merge(a, b));
            }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::CurrencyMismatch);
    }
}
