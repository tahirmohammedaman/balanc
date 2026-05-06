//! `ast::Module` -> `SymbolId`-resolved names. No types yet — that's `typeck`'s job;
//! this stage only answers "which binding does this name refer to?" and rejects names
//! that refer to nothing.
//!
//! Scope is flat and per-transaction (D-013: transactions are closed, so nothing
//! crosses a transaction boundary) and shadowing is allowed: a second `let` with a
//! reused name simply makes later references resolve to the new binding. If the
//! shadowed binding was never consumed, `typeck`'s residual-context check reports that
//! on its own as `E_DROPPED` — resolve does not need a separate duplicate-binding
//! diagnostic for it.

use std::collections::HashMap;

use crate::amount::Amount;
use crate::diag::{Code, Diagnostic};
use crate::parse::ast::{self, AccountPath};
use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SymbolId(pub u32);

pub struct ResolvedModule {
    pub txns: Vec<ResolvedTxn>,
}

pub struct ResolvedTxn {
    pub name: String,
    pub stmts: Vec<ResolvedStmt>,
    pub span: Span,
}

pub enum ResolvedStmt {
    Let { symbol: SymbolId, name_span: Span, value: ResolvedMoneyExpr, span: Span },
    Debit { account: AccountPath, value: ResolvedMoneyExpr, span: Span },
}

pub enum ResolvedMoneyExpr {
    Credit { account: AccountPath, amount: Amount, span: Span },
    Var { symbol: SymbolId, span: Span },
}

pub fn resolve(module: ast::Module) -> (Option<ResolvedModule>, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let mut next_id = 0u32;
    let mut txns = Vec::new();

    for txn in module.txns {
        let mut scope: HashMap<String, SymbolId> = HashMap::new();
        let mut stmts = Vec::new();

        for stmt in txn.stmts {
            match stmt {
                ast::Stmt::Let { name, name_span, value, span } => {
                    let value = resolve_money_expr(value, &scope, &mut diags);
                    let symbol = SymbolId(next_id);
                    next_id += 1;
                    scope.insert(name, symbol);
                    if let Some(value) = value {
                        stmts.push(ResolvedStmt::Let { symbol, name_span, value, span });
                    }
                }
                ast::Stmt::Debit { account, value, span } => {
                    if let Some(value) = resolve_money_expr(value, &scope, &mut diags) {
                        stmts.push(ResolvedStmt::Debit { account, value, span });
                    }
                }
            }
        }

        txns.push(ResolvedTxn { name: txn.name, stmts, span: txn.span });
    }

    if diags.is_empty() {
        (Some(ResolvedModule { txns }), diags)
    } else {
        (None, diags)
    }
}

fn resolve_money_expr(
    expr: ast::MoneyExpr,
    scope: &HashMap<String, SymbolId>,
    diags: &mut Vec<Diagnostic>,
) -> Option<ResolvedMoneyExpr> {
    match expr {
        ast::MoneyExpr::Credit { account, amount, span } => {
            Some(ResolvedMoneyExpr::Credit { account, amount, span })
        }
        ast::MoneyExpr::Var { name, span } => match scope.get(&name) {
            Some(&symbol) => Some(ResolvedMoneyExpr::Var { symbol, span }),
            None => {
                diags.push(Diagnostic::new(
                    Code::UnboundName,
                    format!("no binding named '{name}' in scope"),
                    span,
                ));
                None
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::lex;
    use crate::parse::parse;

    fn resolve_src(src: &str) -> (Option<ResolvedModule>, Vec<Diagnostic>) {
        let (tokens, _) = lex(src);
        let (module, _) = parse(&tokens.unwrap());
        resolve(module.unwrap())
    }

    #[test]
    fn resolves_let_and_var() {
        let (module, diags) = resolve_src(
            r#"txn "coffee" { let m = credit(assets:cash, 45.00); debit(expenses:coffee, m); }"#,
        );
        assert!(diags.is_empty());
        let module = module.unwrap();
        let ResolvedStmt::Let { symbol: bound, .. } = &module.txns[0].stmts[0] else {
            panic!("expected a let");
        };
        let ResolvedStmt::Debit { value: ResolvedMoneyExpr::Var { symbol: used, .. }, .. } =
            &module.txns[0].stmts[1]
        else {
            panic!("expected a debit of a variable");
        };
        assert_eq!(bound, used);
    }

    #[test]
    fn unbound_name_is_reported() {
        let (module, diags) = resolve_src(r#"txn "coffee" { debit(expenses:coffee, m); }"#);
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::UnboundName);
    }

    #[test]
    fn shadowing_reuses_the_name_for_later_references() {
        let (module, diags) = resolve_src(
            r#"txn "t" {
                let m = credit(assets:cash, 10.00);
                let m = credit(assets:cash, 20.00);
                debit(expenses:x, m);
            }"#,
        );
        assert!(diags.is_empty());
        let module = module.unwrap();
        let ResolvedStmt::Let { symbol: second, .. } = &module.txns[0].stmts[1] else {
            panic!("expected the second let");
        };
        let ResolvedStmt::Debit { value: ResolvedMoneyExpr::Var { symbol: used, .. }, .. } =
            &module.txns[0].stmts[2]
        else {
            panic!("expected a debit of a variable");
        };
        assert_eq!(second, used, "debit should resolve to the second (shadowing) binding");
    }
}
