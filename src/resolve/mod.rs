//! `ast::Module` -> id-resolved names. No linearity or currency-*matching* checking
//! yet (that's `typeck`'s job); this stage answers three name-resolution questions:
//! "which currency does this name refer to?", "which declared account does this path
//! refer to?", and "which `let` binding does this `Var` refer to?" — rejecting a name
//! that refers to nothing in each case.
//!
//! Currencies and accounts are resolved once, module-wide, before any transaction body
//! is walked (D-022): a `credit`/`debit`'s account must already be declared, since
//! Slice 2 infers a value's currency from the account it's credited from, and there is
//! nothing to infer from an undeclared one.
//!
//! Scope inside a transaction body is flat and per-transaction (D-013: transactions
//! are closed, so nothing crosses a transaction boundary) and shadowing is allowed: a
//! second `let` with a reused name simply makes later references resolve to the new
//! binding. If the shadowed binding was never consumed, `typeck`'s residual-context
//! check reports that on its own as `E_DROPPED` — resolve does not need a separate
//! duplicate-binding diagnostic for it (D-020).

use std::collections::HashMap;

use crate::diag::{Code, Diagnostic};
use crate::parse::ast::{self, AccountPath, DecimalLiteral};
use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CurrencyId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountId(pub u32);

pub struct CurrencyInfo {
    pub name: String,
    pub scale: u32,
}

pub struct AccountInfo {
    /// Rendered form, e.g. `"assets:cash"` — used in diagnostics and reports.
    pub path: String,
    pub currency: CurrencyId,
}

pub struct ResolvedModule {
    pub currencies: Vec<CurrencyInfo>,
    pub accounts: Vec<AccountInfo>,
    pub txns: Vec<ResolvedTxn>,
}

pub struct ResolvedTxn {
    pub name: String,
    pub stmts: Vec<ResolvedStmt>,
    pub span: Span,
}

pub enum ResolvedStmt {
    Let { symbol: SymbolId, name_span: Span, value: ResolvedMoneyExpr, span: Span },
    Debit { account: AccountId, value: ResolvedMoneyExpr, span: Span },
}

pub enum ResolvedMoneyExpr {
    Credit { account: AccountId, amount: DecimalLiteral, span: Span },
    Var { symbol: SymbolId, span: Span },
}

pub fn resolve(module: ast::Module) -> (Option<ResolvedModule>, Vec<Diagnostic>) {
    let mut diags = Vec::new();

    let (currencies, currency_ids) = resolve_currencies(module.currencies, &mut diags);
    let (accounts, account_ids) =
        resolve_accounts(module.accounts, &currency_ids, &mut diags);

    let mut next_id = 0u32;
    let mut txns = Vec::new();

    for txn in module.txns {
        let mut scope: HashMap<String, SymbolId> = HashMap::new();
        let mut stmts = Vec::new();

        for stmt in txn.stmts {
            match stmt {
                ast::Stmt::Let { name, name_span, value, span } => {
                    let value =
                        resolve_money_expr(value, &scope, &account_ids, &mut diags);
                    let symbol = SymbolId(next_id);
                    next_id += 1;
                    scope.insert(name, symbol);
                    if let Some(value) = value {
                        stmts.push(ResolvedStmt::Let { symbol, name_span, value, span });
                    }
                }
                ast::Stmt::Debit { account, value, span } => {
                    let value =
                        resolve_money_expr(value, &scope, &account_ids, &mut diags);
                    let account = resolve_account_ref(&account, &account_ids, &mut diags);
                    if let (Some(account), Some(value)) = (account, value) {
                        stmts.push(ResolvedStmt::Debit { account, value, span });
                    }
                }
            }
        }

        txns.push(ResolvedTxn { name: txn.name, stmts, span: txn.span });
    }

    if diags.is_empty() {
        (Some(ResolvedModule { currencies, accounts, txns }), diags)
    } else {
        (None, diags)
    }
}

fn resolve_currencies(
    decls: Vec<ast::CurrencyDecl>,
    diags: &mut Vec<Diagnostic>,
) -> (Vec<CurrencyInfo>, HashMap<String, (CurrencyId, Span)>) {
    let mut currencies = Vec::new();
    let mut ids: HashMap<String, (CurrencyId, Span)> = HashMap::new();

    for decl in decls {
        if let Some(&(_, first_span)) = ids.get(&decl.name) {
            diags.push(
                Diagnostic::new(
                    Code::DuplicateCurrency,
                    format!("currency '{}' is already declared", decl.name),
                    decl.name_span,
                )
                .with_secondary("first declared here", first_span),
            );
            continue;
        }
        let id = CurrencyId(currencies.len() as u32);
        ids.insert(decl.name.clone(), (id, decl.name_span));
        currencies.push(CurrencyInfo { name: decl.name, scale: decl.scale });
    }

    (currencies, ids)
}

fn resolve_accounts(
    decls: Vec<ast::AccountDecl>,
    currency_ids: &HashMap<String, (CurrencyId, Span)>,
    diags: &mut Vec<Diagnostic>,
) -> (Vec<AccountInfo>, HashMap<String, (AccountId, Span)>) {
    let mut accounts = Vec::new();
    let mut ids: HashMap<String, (AccountId, Span)> = HashMap::new();

    for decl in decls {
        let path = decl.path.to_string();
        if let Some(&(_, first_span)) = ids.get(&path) {
            diags.push(
                Diagnostic::new(
                    Code::DuplicateAccount,
                    format!("account '{path}' is already declared"),
                    decl.path.span,
                )
                .with_secondary("first declared here", first_span),
            );
            continue;
        }
        let Some(&(currency, _)) = currency_ids.get(&decl.currency) else {
            diags.push(Diagnostic::new(
                Code::UnknownCurrency,
                format!("no currency named '{}' is declared", decl.currency),
                decl.currency_span,
            ));
            continue;
        };
        let id = AccountId(accounts.len() as u32);
        ids.insert(path.clone(), (id, decl.path.span));
        accounts.push(AccountInfo { path, currency });
    }

    (accounts, ids)
}

fn resolve_account_ref(
    account: &AccountPath,
    account_ids: &HashMap<String, (AccountId, Span)>,
    diags: &mut Vec<Diagnostic>,
) -> Option<AccountId> {
    match account_ids.get(&account.to_string()) {
        Some(&(id, _)) => Some(id),
        None => {
            diags.push(Diagnostic::new(
                Code::UndeclaredAccount,
                format!("no account named '{account}' is declared"),
                account.span,
            ));
            None
        }
    }
}

fn resolve_money_expr(
    expr: ast::MoneyExpr,
    scope: &HashMap<String, SymbolId>,
    account_ids: &HashMap<String, (AccountId, Span)>,
    diags: &mut Vec<Diagnostic>,
) -> Option<ResolvedMoneyExpr> {
    match expr {
        ast::MoneyExpr::Credit { account, amount, span } => {
            let account = resolve_account_ref(&account, account_ids, diags)?;
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

    const PRELUDE: &str = r#"
        currency ETB { scale = 2 }
        account assets:cash { currency = ETB }
        account expenses:coffee { currency = ETB }
        account expenses:a { currency = ETB }
        account expenses:b { currency = ETB }
    "#;

    #[test]
    fn resolves_let_and_var() {
        let (module, diags) = resolve_src(&format!(
            r#"{PRELUDE} txn "coffee" {{ let m = credit(assets:cash, 45.00); debit(expenses:coffee, m); }}"#
        ));
        assert!(diags.is_empty(), "{diags:?}");
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
        let (module, diags) =
            resolve_src(&format!(r#"{PRELUDE} txn "coffee" {{ debit(expenses:coffee, m); }}"#));
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::UnboundName);
    }

    #[test]
    fn shadowing_reuses_the_name_for_later_references() {
        let (module, diags) = resolve_src(&format!(
            r#"{PRELUDE} txn "t" {{
                let m = credit(assets:cash, 10.00);
                let m = credit(assets:cash, 20.00);
                debit(expenses:a, m);
            }}"#
        ));
        assert!(diags.is_empty(), "{diags:?}");
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

    #[test]
    fn undeclared_account_is_reported() {
        let (module, diags) = resolve_src(
            r#"currency ETB { scale = 2 }
               txn "coffee" { debit(expenses:coffee, credit(assets:cash, 45.00)); }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::UndeclaredAccount);
    }

    #[test]
    fn account_naming_unknown_currency_is_reported() {
        let (module, diags) = resolve_src(r#"account assets:cash { currency = USD } txn "t" { }"#);
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::UnknownCurrency);
    }

    #[test]
    fn duplicate_currency_is_reported() {
        let (module, diags) = resolve_src(
            r#"currency ETB { scale = 2 }
               currency ETB { scale = 0 }
               txn "t" { }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::DuplicateCurrency);
    }

    #[test]
    fn duplicate_account_is_reported() {
        let (module, diags) = resolve_src(
            r#"currency ETB { scale = 2 }
               account assets:cash { currency = ETB }
               account assets:cash { currency = ETB }
               txn "t" { }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::DuplicateAccount);
    }
}
