//! `resolve::ResolvedModule` -> `TModule` + `Diagnostic`s. The traversal driver: walks
//! each transaction body left to right, threading Γ (`rules::Context`) through `let`
//! and `debit` statements per the rules documented in `rules.rs`.

pub mod rules;

use crate::amount::Amount;
use crate::diag::{Code, Diagnostic};
use crate::resolve::{
    AccountId, AccountInfo, CurrencyId, CurrencyInfo, ResolvedModule, ResolvedMoneyExpr,
    ResolvedStmt, ResolvedTxn, SymbolId,
};
use crate::span::Span;
use rules::Context;
pub use rules::Ty;

pub struct TModule {
    pub currencies: Vec<CurrencyInfo>,
    pub accounts: Vec<AccountInfo>,
    pub txns: Vec<TTxnDecl>,
}

pub struct TTxnDecl {
    pub name: String,
    pub stmts: Vec<TStmt>,
    pub span: Span,
}

pub enum TStmt {
    Let { symbol: SymbolId, value: TMoneyExpr, span: Span },
    Debit { account: AccountId, value: TMoneyExpr, span: Span },
}

pub enum TMoneyExpr {
    Credit { account: AccountId, amount: Amount },
    Var { symbol: SymbolId },
}

pub fn typeck(module: ResolvedModule) -> (Option<TModule>, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let txns = module
        .txns
        .into_iter()
        .map(|txn| typeck_txn(txn, &module.accounts, &module.currencies, &mut diags))
        .collect();
    if diags.is_empty() {
        (Some(TModule { currencies: module.currencies, accounts: module.accounts, txns }), diags)
    } else {
        (None, diags)
    }
}

fn typeck_txn(
    txn: ResolvedTxn,
    accounts: &[AccountInfo],
    currencies: &[CurrencyInfo],
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
                ctx.bind(symbol, name_span, currency_span, currency);
                Some(TStmt::Let { symbol, value, span })
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
        })
        .collect();
    diags.extend(ctx.finish_txn());
    TTxnDecl { name: txn.name, stmts, span: txn.span }
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
            let amount = match Amount::from_literal(&amount.text, scale) {
                Ok(amount) => amount,
                Err(frac_digits) => {
                    diags.push(Diagnostic::new(
                        Code::TooManyFractionDigits,
                        format!(
                            "amount has {frac_digits} fractional digits, but currency '{}' has scale {scale}",
                            currencies[currency.0 as usize].name
                        ),
                        amount.span,
                    ));
                    return None;
                }
            };
            Some((TMoneyExpr::Credit { account, amount }, currency, span))
        }
        ResolvedMoneyExpr::Var { symbol, span } => match ctx.consume(symbol, span) {
            Ok((currency_span, currency)) => {
                Some((TMoneyExpr::Var { symbol }, currency, currency_span))
            }
            Err(d) => {
                diags.push(d);
                None
            }
        },
    }
}

fn currency_mismatch(
    account: &AccountInfo,
    currencies: &[CurrencyInfo],
    found: CurrencyId,
    found_span: Span,
    debit_span: Span,
) -> Diagnostic {
    let expected_name = &currencies[account.currency.0 as usize].name;
    let found_name = &currencies[found.0 as usize].name;
    Diagnostic::new(
        Code::CurrencyMismatch,
        format!(
            "account '{}' expects currency '{expected_name}', but this money is '{found_name}'",
            account.path
        ),
        debit_span,
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
        assert_eq!(diags[0].secondary[0].0, "this money's currency was established here");
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
}
