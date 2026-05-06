//! `resolve::ResolvedModule` -> `TModule` + `Diagnostic`s. The traversal driver: walks
//! each transaction body left to right, threading Γ (`rules::Context`) through `let`
//! and `debit` statements per the rules documented in `rules.rs`.

pub mod rules;

use crate::amount::Amount;
use crate::diag::Diagnostic;
use crate::parse::ast::AccountPath;
use crate::resolve::{ResolvedModule, ResolvedMoneyExpr, ResolvedStmt, ResolvedTxn, SymbolId};
use crate::span::Span;
use rules::Context;
pub use rules::Ty;

pub struct TModule {
    pub txns: Vec<TTxnDecl>,
}

pub struct TTxnDecl {
    pub name: String,
    pub stmts: Vec<TStmt>,
    pub span: Span,
}

pub enum TStmt {
    Let { symbol: SymbolId, value: TMoneyExpr, span: Span },
    Debit { account: AccountPath, value: TMoneyExpr, span: Span },
}

pub enum TMoneyExpr {
    Credit { account: AccountPath, amount: Amount },
    Var { symbol: SymbolId },
}

pub fn typeck(module: ResolvedModule) -> (Option<TModule>, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let txns = module.txns.into_iter().map(|txn| typeck_txn(txn, &mut diags)).collect();
    if diags.is_empty() {
        (Some(TModule { txns }), diags)
    } else {
        (None, diags)
    }
}

fn typeck_txn(txn: ResolvedTxn, diags: &mut Vec<Diagnostic>) -> TTxnDecl {
    let mut ctx = Context::default();
    let stmts = txn
        .stmts
        .into_iter()
        .map(|stmt| match stmt {
            ResolvedStmt::Let { symbol, name_span, value, span } => {
                let value = typeck_money_expr(value, &mut ctx, diags);
                ctx.bind(symbol, name_span);
                TStmt::Let { symbol, value, span }
            }
            ResolvedStmt::Debit { account, value, span } => {
                let value = typeck_money_expr(value, &mut ctx, diags);
                TStmt::Debit { account, value, span }
            }
        })
        .collect();
    diags.extend(ctx.finish_txn());
    TTxnDecl { name: txn.name, stmts, span: txn.span }
}

fn typeck_money_expr(
    expr: ResolvedMoneyExpr,
    ctx: &mut Context,
    diags: &mut Vec<Diagnostic>,
) -> TMoneyExpr {
    match expr {
        ResolvedMoneyExpr::Credit { account, amount, .. } => TMoneyExpr::Credit { account, amount },
        ResolvedMoneyExpr::Var { symbol, span } => {
            if let Err(d) = ctx.consume(symbol, span) {
                diags.push(d);
            }
            TMoneyExpr::Var { symbol }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Code;
    use crate::lex::lex;
    use crate::parse::parse;
    use crate::resolve::resolve;

    fn typeck_src(src: &str) -> (Option<TModule>, Vec<Diagnostic>) {
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
}
