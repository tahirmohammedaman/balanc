//! `ast::Module` -> id-resolved names. No linearity or currency-*matching* checking
//! yet (that's `typeck`'s job); this stage answers four name-resolution questions:
//! "which currency does this name refer to?", "which declared account does this path
//! refer to?", "which declared rate does this name refer to?", and "which `let` (or
//! `convert`) binding does this name refer to?" — rejecting a name that refers to
//! nothing in each case.
//!
//! Currencies, accounts, and rates are resolved once, module-wide, before any
//! transaction body is walked (D-022): a `credit`/`debit`'s account must already be
//! declared, since Slice 2 infers a value's currency from the account it's credited
//! from, and there is nothing to infer from an undeclared one. Rates follow the same
//! pattern for `convert` (Slice 3).
//!
//! Scope inside a transaction body is flat and per-transaction (D-013: transactions
//! are closed, so nothing crosses a transaction boundary) and shadowing is allowed: a
//! second `let` with a reused name simply makes later references resolve to the new
//! binding. If the shadowed binding was never consumed, `typeck`'s residual-context
//! check reports that on its own as `E_DROPPED` — resolve does not need a separate
//! duplicate-binding diagnostic for it (D-020). `convert`'s two bindings (a `Money`
//! and a `Residue`, D-026) go into the same flat scope as an ordinary `let` — resolve
//! doesn't distinguish the two kinds of binding at all; that distinction (and the
//! `E_EXPECTED_MONEY`/`E_EXPECTED_RESIDUE` diagnostics it drives) is `typeck`'s job,
//! once each binding's kind is actually known.

use std::collections::HashMap;

use crate::diag::{Code, Diagnostic};
use crate::parse::ast::{self, AccountPath, DecimalLiteral, NormalBalance};
use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CurrencyId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RateId(pub u32);

pub struct CurrencyInfo {
    pub name: String,
    pub scale: u32,
}

pub struct AccountInfo {
    /// Rendered form, e.g. `"assets:cash"` — used in diagnostics and reports.
    pub path: String,
    pub currency: CurrencyId,
    /// Report-time category (Slice 5) — drives `render`'s section grouping.
    pub kind: AccountKind,
    /// Which side of a debit/credit entry this account's balance is conventionally
    /// displayed positive on (Slice 5) — independent of `kind`, so a contra account
    /// (e.g. an asset-kind account with a credit normal balance) is representable.
    pub normal: NormalBalance,
}

/// The five closed report categories a Slice 5 `account` declaration's `kind` field
/// may name. Unlike `CurrencyId`/`AccountId`/`RateId`, this isn't a table of
/// user-declared entries — it's a fixed, built-in enumeration, so resolving a `kind`
/// string means matching it against these five spellings rather than looking it up in
/// a `HashMap` built from the module (D-036).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}

/// A declared conversion rate. `numerator`/`scale` are the rate's own literal lowered
/// via `amount::parse_fixed_point` (D-027) — a rate has no currency-derived scale
/// ceiling to check against, unlike an `Amount` literal, so it's just accepted as
/// written. The rounding mode isn't stored: `round down` is required syntax (D-016)
/// but `down` is the only value that ever type-checks (D-027), so there's nothing to
/// branch on yet.
pub struct RateInfo {
    pub name: String,
    pub from: CurrencyId,
    pub to: CurrencyId,
    pub numerator: i64,
    pub scale: u32,
}

pub struct ResolvedModule {
    pub currencies: Vec<CurrencyInfo>,
    pub accounts: Vec<AccountInfo>,
    pub rates: Vec<RateInfo>,
    pub txns: Vec<ResolvedTxn>,
}

pub struct ResolvedTxn {
    pub name: String,
    pub stmts: Vec<ResolvedStmt>,
    pub span: Span,
}

pub enum ResolvedStmt {
    Let { symbol: SymbolId, name_span: Span, value: ResolvedMoneyExpr, span: Span },
    /// `let (primary, residual) = convert(money, rate);` — see D-026.
    Convert {
        primary: SymbolId,
        primary_span: Span,
        residual: SymbolId,
        residual_span: Span,
        money: ResolvedMoneyExpr,
        rate: RateId,
        span: Span,
    },
    Debit { account: AccountId, value: ResolvedMoneyExpr, span: Span },
    /// `absorb(residue, account);` — see D-028. `residue` is resolved the same way a
    /// `Var` is (a name looked up in the flat per-transaction scope); which *kind* of
    /// binding it names (a `Money` or a `Residue`) is checked by `typeck`.
    Absorb { residue: SymbolId, residue_span: Span, account: AccountId, span: Span },
    /// `let (a, b) = split(money, amount);` — see D-034. `amount` stays a raw literal
    /// for the same reason `credit`'s does (D-024): lowering it needs a scale, only
    /// known once `money`'s currency is known, which `typeck` determines.
    Split {
        a: SymbolId,
        a_span: Span,
        b: SymbolId,
        b_span: Span,
        money: ResolvedMoneyExpr,
        amount: DecimalLiteral,
        span: Span,
    },
    /// `let (a, b) = split_ratio(money, a_weight, b_weight);` — see D-015/D-035. The
    /// weights are already whole numbers straight from the parser (D-035), so unlike
    /// `amount` above there's no currency-scale lowering step to defer.
    SplitRatio {
        a: SymbolId,
        a_span: Span,
        b: SymbolId,
        b_span: Span,
        money: ResolvedMoneyExpr,
        a_weight: u32,
        a_weight_span: Span,
        b_weight: u32,
        b_weight_span: Span,
        span: Span,
    },
}

pub enum ResolvedMoneyExpr {
    Credit { account: AccountId, amount: DecimalLiteral, span: Span },
    Var { symbol: SymbolId, span: Span },
    /// `merge(a, b)` — see T-Merge. Resolving `a` then `b` in sequence (rather than,
    /// say, resolving both against independent copies of `scope`) is what's needed
    /// for `typeck`'s later `merge(m, m)` -> `E_REUSED` check to work at all — but
    /// that check only concerns *linear consumption*, which resolve doesn't track;
    /// resolve just needs both sub-expressions' names to exist in scope, same as any
    /// other `MoneyExpr` position.
    Merge { a: Box<ResolvedMoneyExpr>, b: Box<ResolvedMoneyExpr>, span: Span },
}

pub fn resolve(module: ast::Module) -> (Option<ResolvedModule>, Vec<Diagnostic>) {
    let mut diags = Vec::new();

    let (currencies, currency_ids) = resolve_currencies(module.currencies, &mut diags);
    let (accounts, account_ids) = resolve_accounts(module.accounts, &currency_ids, &mut diags);
    let (rates, rate_ids) = resolve_rates(module.rates, &currency_ids, &mut diags);

    let mut next_id = 0u32;
    let mut txns = Vec::new();

    for txn in module.txns {
        let mut scope: HashMap<String, SymbolId> = HashMap::new();
        let mut stmts = Vec::new();
        let mut fresh_symbol = || {
            let symbol = SymbolId(next_id);
            next_id += 1;
            symbol
        };

        for stmt in txn.stmts {
            match stmt {
                ast::Stmt::Let { name, name_span, value, span } => {
                    let value = resolve_money_expr(value, &scope, &account_ids, &mut diags);
                    let symbol = fresh_symbol();
                    scope.insert(name, symbol);
                    if let Some(value) = value {
                        stmts.push(ResolvedStmt::Let { symbol, name_span, value, span });
                    }
                }
                ast::Stmt::Convert {
                    primary_name,
                    primary_span,
                    residual_name,
                    residual_span,
                    money,
                    rate,
                    rate_span,
                    span,
                } => {
                    let money = resolve_money_expr(money, &scope, &account_ids, &mut diags);
                    let rate_id = resolve_rate_ref(&rate, rate_span, &rate_ids, &mut diags);
                    let primary = fresh_symbol();
                    scope.insert(primary_name, primary);
                    let residual = fresh_symbol();
                    scope.insert(residual_name, residual);
                    if let (Some(money), Some(rate)) = (money, rate_id) {
                        stmts.push(ResolvedStmt::Convert {
                            primary,
                            primary_span,
                            residual,
                            residual_span,
                            money,
                            rate,
                            span,
                        });
                    }
                }
                ast::Stmt::Debit { account, value, span } => {
                    let value = resolve_money_expr(value, &scope, &account_ids, &mut diags);
                    let account = resolve_account_ref(&account, &account_ids, &mut diags);
                    if let (Some(account), Some(value)) = (account, value) {
                        stmts.push(ResolvedStmt::Debit { account, value, span });
                    }
                }
                ast::Stmt::Absorb { residue_name, residue_span, account, span } => {
                    let residue = resolve_var_ref(&residue_name, residue_span, &scope, &mut diags);
                    let account = resolve_account_ref(&account, &account_ids, &mut diags);
                    if let (Some(residue), Some(account)) = (residue, account) {
                        stmts.push(ResolvedStmt::Absorb { residue, residue_span, account, span });
                    }
                }
                ast::Stmt::Split { a_name, a_span, b_name, b_span, money, amount, span } => {
                    let money = resolve_money_expr(money, &scope, &account_ids, &mut diags);
                    let a = fresh_symbol();
                    scope.insert(a_name, a);
                    let b = fresh_symbol();
                    scope.insert(b_name, b);
                    if let Some(money) = money {
                        stmts.push(ResolvedStmt::Split {
                            a,
                            a_span,
                            b,
                            b_span,
                            money,
                            amount,
                            span,
                        });
                    }
                }
                ast::Stmt::SplitRatio {
                    a_name,
                    a_span,
                    b_name,
                    b_span,
                    money,
                    a_weight,
                    a_weight_span,
                    b_weight,
                    b_weight_span,
                    span,
                } => {
                    let money = resolve_money_expr(money, &scope, &account_ids, &mut diags);
                    let a = fresh_symbol();
                    scope.insert(a_name, a);
                    let b = fresh_symbol();
                    scope.insert(b_name, b);
                    if let Some(money) = money {
                        stmts.push(ResolvedStmt::SplitRatio {
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
                        });
                    }
                }
            }
        }

        txns.push(ResolvedTxn { name: txn.name, stmts, span: txn.span });
    }

    if diags.is_empty() {
        (Some(ResolvedModule { currencies, accounts, rates, txns }), diags)
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
        let Some(kind) = resolve_account_kind(&decl.kind, decl.kind_span, diags) else {
            continue;
        };
        let id = AccountId(accounts.len() as u32);
        ids.insert(path.clone(), (id, decl.path.span));
        accounts.push(AccountInfo { path, currency, kind, normal: decl.normal });
    }

    (accounts, ids)
}

fn resolve_rates(
    decls: Vec<ast::RateDecl>,
    currency_ids: &HashMap<String, (CurrencyId, Span)>,
    diags: &mut Vec<Diagnostic>,
) -> (Vec<RateInfo>, HashMap<String, (RateId, Span)>) {
    let mut rates = Vec::new();
    let mut ids: HashMap<String, (RateId, Span)> = HashMap::new();

    for decl in decls {
        if let Some(&(_, first_span)) = ids.get(&decl.name) {
            diags.push(
                Diagnostic::new(
                    Code::DuplicateRate,
                    format!("rate '{}' is already declared", decl.name),
                    decl.name_span,
                )
                .with_secondary("first declared here", first_span),
            );
            continue;
        }
        let Some(&(from, _)) = currency_ids.get(&decl.from) else {
            diags.push(Diagnostic::new(
                Code::UnknownCurrency,
                format!("no currency named '{}' is declared", decl.from),
                decl.from_span,
            ));
            continue;
        };
        let Some(&(to, _)) = currency_ids.get(&decl.to) else {
            diags.push(Diagnostic::new(
                Code::UnknownCurrency,
                format!("no currency named '{}' is declared", decl.to),
                decl.to_span,
            ));
            continue;
        };
        let Some((numerator, scale)) = crate::amount::parse_fixed_point(&decl.value.text) else {
            diags.push(Diagnostic::new(
                Code::AmountOutOfRange,
                "this rate is too large to represent",
                decl.value.span,
            ));
            continue;
        };
        let id = RateId(rates.len() as u32);
        ids.insert(decl.name.clone(), (id, decl.name_span));
        rates.push(RateInfo { name: decl.name, from, to, numerator, scale });
    }

    (rates, ids)
}

/// Matches an `account` declaration's `kind` text against the five closed values —
/// see `AccountKind`'s doc comment for why this is a fixed match, not a table lookup.
fn resolve_account_kind(
    text: &str,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) -> Option<AccountKind> {
    match text {
        "asset" => Some(AccountKind::Asset),
        "liability" => Some(AccountKind::Liability),
        "equity" => Some(AccountKind::Equity),
        "income" => Some(AccountKind::Income),
        "expense" => Some(AccountKind::Expense),
        _ => {
            diags.push(Diagnostic::new(
                Code::UnknownAccountKind,
                format!(
                    "no such account kind '{text}' (expected one of: asset, liability, equity, income, expense)"
                ),
                span,
            ));
            None
        }
    }
}

fn resolve_rate_ref(
    name: &str,
    span: Span,
    rate_ids: &HashMap<String, (RateId, Span)>,
    diags: &mut Vec<Diagnostic>,
) -> Option<RateId> {
    match rate_ids.get(name) {
        Some(&(id, _)) => Some(id),
        None => {
            diags.push(Diagnostic::new(
                Code::UndeclaredRate,
                format!("no rate named '{name}' is declared"),
                span,
            ));
            None
        }
    }
}

fn resolve_var_ref(
    name: &str,
    span: Span,
    scope: &HashMap<String, SymbolId>,
    diags: &mut Vec<Diagnostic>,
) -> Option<SymbolId> {
    match scope.get(name) {
        Some(&symbol) => Some(symbol),
        None => {
            diags.push(Diagnostic::new(
                Code::UnboundName,
                format!("no binding named '{name}' in scope"),
                span,
            ));
            None
        }
    }
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
        ast::MoneyExpr::Var { name, span } => {
            let symbol = resolve_var_ref(&name, span, scope, diags)?;
            Some(ResolvedMoneyExpr::Var { symbol, span })
        }
        ast::MoneyExpr::Merge { a, b, span } => {
            // Resolve both sides unconditionally (not `a?` then `b`) so an unresolved
            // name on one side doesn't hide diagnostics from the other, matching every
            // other two-sided statement in this module (e.g. `Debit`'s account/value).
            let a = resolve_money_expr(*a, scope, account_ids, diags);
            let b = resolve_money_expr(*b, scope, account_ids, diags);
            match (a, b) {
                (Some(a), Some(b)) => {
                    Some(ResolvedMoneyExpr::Merge { a: Box::new(a), b: Box::new(b), span })
                }
                _ => None,
            }
        }
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
        account assets:cash { currency = ETB, kind = asset, normal = debit }
        account expenses:coffee { currency = ETB, kind = expense, normal = debit }
        account expenses:a { currency = ETB, kind = expense, normal = debit }
        account expenses:b { currency = ETB, kind = expense, normal = debit }
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
        let (module, diags) = resolve_src(
            r#"account assets:cash { currency = USD, kind = asset, normal = debit } txn "t" { }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::UnknownCurrency);
    }

    #[test]
    fn account_naming_unknown_kind_is_reported() {
        let (module, diags) = resolve_src(
            r#"currency ETB { scale = 2 }
               account assets:cash { currency = ETB, kind = bogus, normal = debit }
               txn "t" { }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::UnknownAccountKind);
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
               account assets:cash { currency = ETB, kind = asset, normal = debit }
               account assets:cash { currency = ETB, kind = asset, normal = debit }
               txn "t" { }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::DuplicateAccount);
    }

    const FX_PRELUDE: &str = r#"
        currency USD { scale = 2 }
        currency ETB { scale = 2 }
        account assets:usd_cash { currency = USD, kind = asset, normal = debit }
        account assets:etb_cash { currency = ETB, kind = asset, normal = debit }
        account income:fx_rounding { currency = ETB, kind = income, normal = credit }
        rate usd_etb from USD to ETB = 57.20 round down;
    "#;

    #[test]
    fn resolves_convert_and_absorb() {
        let (module, diags) = resolve_src(&format!(
            r#"{FX_PRELUDE} txn "fx" {{
                let m = credit(assets:usd_cash, 100.00);
                let (m2, r) = convert(m, usd_etb);
                debit(assets:etb_cash, m2);
                absorb(r, income:fx_rounding);
            }}"#
        ));
        assert!(diags.is_empty(), "{diags:?}");
        let module = module.unwrap();
        assert_eq!(module.rates.len(), 1);
        let stmts = &module.txns[0].stmts;
        let ResolvedStmt::Convert { primary, residual, .. } = &stmts[1] else {
            panic!("expected a convert statement");
        };
        let ResolvedStmt::Debit { value: ResolvedMoneyExpr::Var { symbol: debited, .. }, .. } =
            &stmts[2]
        else {
            panic!("expected a debit of a variable");
        };
        assert_eq!(primary, debited);
        let ResolvedStmt::Absorb { residue, .. } = &stmts[3] else {
            panic!("expected an absorb statement");
        };
        assert_eq!(residual, residue);
    }

    #[test]
    fn undeclared_rate_is_reported() {
        let (module, diags) = resolve_src(&format!(
            r#"{FX_PRELUDE} txn "fx" {{
                let m = credit(assets:usd_cash, 100.00);
                let (m2, r) = convert(m, nonexistent_rate);
                debit(assets:etb_cash, m2);
                absorb(r, income:fx_rounding);
            }}"#
        ));
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::UndeclaredRate);
    }

    #[test]
    fn duplicate_rate_is_reported() {
        let (module, diags) = resolve_src(
            r#"currency USD { scale = 2 }
               currency ETB { scale = 2 }
               rate usd_etb from USD to ETB = 57.20 round down;
               rate usd_etb from USD to ETB = 58.00 round down;
               txn "t" { }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::DuplicateRate);
    }

    #[test]
    fn rate_naming_unknown_currency_is_reported() {
        let (module, diags) = resolve_src(
            r#"currency USD { scale = 2 }
               rate usd_etb from USD to ETB = 57.20 round down;
               txn "t" { }"#,
        );
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::UnknownCurrency);
    }

    #[test]
    fn resolves_split() {
        let (module, diags) = resolve_src(&format!(
            r#"{PRELUDE} txn "t" {{
                let m = credit(assets:cash, 45.00);
                let (a, b) = split(m, 20.00);
                debit(expenses:a, a);
                debit(expenses:b, b);
            }}"#
        ));
        assert!(diags.is_empty(), "{diags:?}");
        let module = module.unwrap();
        let stmts = &module.txns[0].stmts;
        let ResolvedStmt::Split { a, b, .. } = &stmts[1] else {
            panic!("expected a split statement");
        };
        let ResolvedStmt::Debit { value: ResolvedMoneyExpr::Var { symbol: used_a, .. }, .. } =
            &stmts[2]
        else {
            panic!("expected a debit of a variable");
        };
        let ResolvedStmt::Debit { value: ResolvedMoneyExpr::Var { symbol: used_b, .. }, .. } =
            &stmts[3]
        else {
            panic!("expected a debit of a variable");
        };
        assert_eq!(a, used_a);
        assert_eq!(b, used_b);
    }

    #[test]
    fn resolves_split_ratio() {
        let (module, diags) = resolve_src(&format!(
            r#"{PRELUDE} txn "t" {{
                let m = credit(assets:cash, 45.00);
                let (a, b) = split_ratio(m, 1, 2);
                debit(expenses:a, a);
                debit(expenses:b, b);
            }}"#
        ));
        assert!(diags.is_empty(), "{diags:?}");
        let module = module.unwrap();
        let ResolvedStmt::SplitRatio { a_weight, b_weight, .. } = &module.txns[0].stmts[1] else {
            panic!("expected a split_ratio statement");
        };
        assert_eq!(*a_weight, 1);
        assert_eq!(*b_weight, 2);
    }

    #[test]
    fn resolves_merge() {
        let (module, diags) = resolve_src(&format!(
            r#"{PRELUDE} txn "t" {{
                let a = credit(assets:cash, 20.00);
                let b = credit(assets:cash, 25.00);
                debit(expenses:coffee, merge(a, b));
            }}"#
        ));
        assert!(diags.is_empty(), "{diags:?}");
        let module = module.unwrap();
        assert!(matches!(
            &module.txns[0].stmts[2],
            ResolvedStmt::Debit { value: ResolvedMoneyExpr::Merge { .. }, .. }
        ));
    }

    #[test]
    fn merge_with_an_unbound_name_is_reported() {
        let (module, diags) = resolve_src(&format!(
            r#"{PRELUDE} txn "t" {{
                let a = credit(assets:cash, 20.00);
                debit(expenses:coffee, merge(a, nonexistent));
            }}"#
        ));
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::UnboundName);
    }
}
