//! `Vec<Token>` -> `ast::Module`, hand-written recursive descent (D-007).
//!
//! Grammar (Slice 3):
//!
//! ```text
//! Module        := Decl* Eof
//! Decl          := CurrencyDecl | AccountDecl | RateDecl | TxnDecl
//! CurrencyDecl  := "currency" Ident "{" "scale" "=" Decimal "}"
//! AccountDecl   := "account" AccountPath "{" "currency" "=" Ident "}"
//! RateDecl      := "rate" Ident "from" Ident "to" Ident "=" Decimal "round" "down" ";"
//! TxnDecl       := "txn" String "{" Stmt* "}"
//! Stmt          := LetStmt | ConvertStmt | DebitStmt | AbsorbStmt
//! LetStmt       := "let" Ident "=" MoneyExpr ";"
//! ConvertStmt   := "let" "(" Ident "," Ident ")" "=" "convert" "(" MoneyExpr "," Ident ")" ";"
//! DebitStmt     := "debit" "(" AccountPath "," MoneyExpr ")" ";"
//! AbsorbStmt    := "absorb" "(" Ident "," AccountPath ")" ";"
//! MoneyExpr     := CreditExpr | Var
//! CreditExpr    := "credit" "(" AccountPath "," Decimal ")"
//! Var           := Ident
//! AccountPath   := Ident (":" Ident)*
//! ```
//!
//! `CurrencyDecl`, `AccountDecl`, `RateDecl`, and `TxnDecl` may appear in any order at
//! module level — nothing here requires currencies before the accounts/rates that use
//! them, or any of those before the transactions that reference them; `resolve` builds
//! all three tables before walking transaction bodies. `ConvertStmt` is distinguished
//! from `LetStmt` by one token of lookahead: `let` followed by `(` is always a
//! `ConvertStmt` (an ordinary `let`'s left-hand side is a single `Ident`, never a
//! parenthesized pair), so no backtracking is needed. `RateDecl`'s `round down` is
//! required syntax even though `down` is the only value that currently type-checks
//! (D-027).
//!
//! No error recovery yet (Slice 6): the first unexpected token stops parsing and
//! returns a single diagnostic, matching the lexer's fatal-error behaviour.

pub mod ast;

use crate::diag::{Code, Diagnostic};
use crate::lex::{Token, TokenKind};
use crate::span::Span;
use ast::{
    AccountDecl, AccountPath, CurrencyDecl, DecimalLiteral, Module, MoneyExpr, RateDecl, Stmt,
    TxnDecl,
};

pub fn parse(tokens: &[Token]) -> (Option<Module>, Vec<Diagnostic>) {
    let mut p = Parser { tokens, pos: 0 };
    match p.parse_module() {
        Ok(module) => (Some(module), Vec::new()),
        Err(diag) => (None, vec![diag]),
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

type PResult<T> = Result<T, Diagnostic>;

impl<'a> Parser<'a> {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn unexpected(&self, expected: &str) -> Diagnostic {
        let tok = self.peek();
        let (code, found) = match &tok.kind {
            TokenKind::Eof => (Code::ParseUnexpectedEof, "end of input".to_string()),
            other => (Code::ParseUnexpectedToken, describe(other)),
        };
        Diagnostic::new(code, format!("expected {expected}, found {found}"), tok.span)
    }

    fn expect(&mut self, kind: &TokenKind, expected: &str) -> PResult<Span> {
        if self.peek().kind == *kind {
            Ok(self.advance().span)
        } else {
            Err(self.unexpected(expected))
        }
    }

    fn parse_module(&mut self) -> PResult<Module> {
        let mut currencies = Vec::new();
        let mut accounts = Vec::new();
        let mut rates = Vec::new();
        let mut txns = Vec::new();
        while self.peek().kind != TokenKind::Eof {
            match &self.peek().kind {
                TokenKind::KwCurrency => currencies.push(self.parse_currency_decl()?),
                TokenKind::KwAccount => accounts.push(self.parse_account_decl()?),
                TokenKind::KwRate => rates.push(self.parse_rate_decl()?),
                TokenKind::KwTxn => txns.push(self.parse_txn()?),
                _ => return Err(self.unexpected("'currency', 'account', 'rate', or 'txn'")),
            }
        }
        Ok(Module { currencies, accounts, rates, txns })
    }

    fn parse_currency_decl(&mut self) -> PResult<CurrencyDecl> {
        let start = self.expect(&TokenKind::KwCurrency, "'currency'")?;
        let (name, name_span) = self.parse_ident()?;
        self.expect(&TokenKind::LBrace, "'{'")?;
        self.expect(&TokenKind::KwScale, "'scale'")?;
        self.expect(&TokenKind::Eq, "'='")?;
        let (scale, scale_span) = self.parse_scale()?;
        let end = self.expect(&TokenKind::RBrace, "'}'")?;
        Ok(CurrencyDecl { name, name_span, scale, scale_span, span: start.to(end) })
    }

    fn parse_scale(&mut self) -> PResult<(u32, Span)> {
        match &self.peek().kind {
            TokenKind::Decimal(text) if !text.contains('.') => {
                let text = text.clone();
                let span = self.advance().span;
                let scale = text.parse().map_err(|_| {
                    Diagnostic::new(Code::ParseUnexpectedToken, "scale is too large", span)
                })?;
                Ok((scale, span))
            }
            _ => Err(self.unexpected("a whole number")),
        }
    }

    fn parse_account_decl(&mut self) -> PResult<AccountDecl> {
        let start = self.expect(&TokenKind::KwAccount, "'account'")?;
        let path = self.parse_account_path()?;
        self.expect(&TokenKind::LBrace, "'{'")?;
        self.expect(&TokenKind::KwCurrency, "'currency'")?;
        self.expect(&TokenKind::Eq, "'='")?;
        let (currency, currency_span) = self.parse_ident()?;
        let end = self.expect(&TokenKind::RBrace, "'}'")?;
        Ok(AccountDecl { path, currency, currency_span, span: start.to(end) })
    }

    fn parse_rate_decl(&mut self) -> PResult<RateDecl> {
        let start = self.expect(&TokenKind::KwRate, "'rate'")?;
        let (name, name_span) = self.parse_ident()?;
        self.expect(&TokenKind::KwFrom, "'from'")?;
        let (from, from_span) = self.parse_ident()?;
        self.expect(&TokenKind::KwTo, "'to'")?;
        let (to, to_span) = self.parse_ident()?;
        self.expect(&TokenKind::Eq, "'='")?;
        let value = self.parse_amount()?;
        self.expect(&TokenKind::KwRound, "'round'")?;
        self.expect(&TokenKind::KwDown, "'down'")?;
        let end = self.expect(&TokenKind::Semi, "';'")?;
        Ok(RateDecl { name, name_span, from, from_span, to, to_span, value, span: start.to(end) })
    }

    fn parse_txn(&mut self) -> PResult<TxnDecl> {
        let start = self.expect(&TokenKind::KwTxn, "'txn'")?;
        let name = match &self.peek().kind {
            TokenKind::Str(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => return Err(self.unexpected("a transaction name string")),
        };
        self.expect(&TokenKind::LBrace, "'{'")?;
        let mut stmts = Vec::new();
        while self.peek().kind != TokenKind::RBrace {
            if self.peek().kind == TokenKind::Eof {
                return Err(self.unexpected("'}'"));
            }
            stmts.push(self.parse_stmt()?);
        }
        let end = self.expect(&TokenKind::RBrace, "'}'")?;
        Ok(TxnDecl { name, stmts, span: start.to(end) })
    }

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        match &self.peek().kind {
            TokenKind::KwLet => self.parse_let_or_convert(),
            TokenKind::KwDebit => self.parse_debit(),
            TokenKind::KwAbsorb => self.parse_absorb(),
            _ => Err(self.unexpected("'let', 'debit', or 'absorb'")),
        }
    }

    /// `let` followed by `(` is always a `ConvertStmt` — an ordinary `let`'s
    /// left-hand side is a single `Ident`, never a parenthesized pair — so one token
    /// of lookahead after `let` disambiguates without backtracking.
    fn parse_let_or_convert(&mut self) -> PResult<Stmt> {
        let start = self.expect(&TokenKind::KwLet, "'let'")?;
        if self.peek().kind == TokenKind::LParen {
            self.parse_convert(start)
        } else {
            let (name, name_span) = self.parse_ident()?;
            self.expect(&TokenKind::Eq, "'='")?;
            let value = self.parse_money_expr()?;
            let end = self.expect(&TokenKind::Semi, "';'")?;
            Ok(Stmt::Let { name, name_span, value, span: start.to(end) })
        }
    }

    fn parse_convert(&mut self, start: Span) -> PResult<Stmt> {
        self.expect(&TokenKind::LParen, "'('")?;
        let (primary_name, primary_span) = self.parse_ident()?;
        self.expect(&TokenKind::Comma, "','")?;
        let (residual_name, residual_span) = self.parse_ident()?;
        self.expect(&TokenKind::RParen, "')'")?;
        self.expect(&TokenKind::Eq, "'='")?;
        self.expect(&TokenKind::KwConvert, "'convert'")?;
        self.expect(&TokenKind::LParen, "'('")?;
        let money = self.parse_money_expr()?;
        self.expect(&TokenKind::Comma, "','")?;
        let (rate, rate_span) = self.parse_ident()?;
        self.expect(&TokenKind::RParen, "')'")?;
        let end = self.expect(&TokenKind::Semi, "';'")?;
        Ok(Stmt::Convert {
            primary_name,
            primary_span,
            residual_name,
            residual_span,
            money,
            rate,
            rate_span,
            span: start.to(end),
        })
    }

    fn parse_debit(&mut self) -> PResult<Stmt> {
        let start = self.expect(&TokenKind::KwDebit, "'debit'")?;
        self.expect(&TokenKind::LParen, "'('")?;
        let account = self.parse_account_path()?;
        self.expect(&TokenKind::Comma, "','")?;
        let value = self.parse_money_expr()?;
        self.expect(&TokenKind::RParen, "')'")?;
        let end = self.expect(&TokenKind::Semi, "';'")?;
        Ok(Stmt::Debit { account, value, span: start.to(end) })
    }

    fn parse_absorb(&mut self) -> PResult<Stmt> {
        let start = self.expect(&TokenKind::KwAbsorb, "'absorb'")?;
        self.expect(&TokenKind::LParen, "'('")?;
        let (residue_name, residue_span) = self.parse_ident()?;
        self.expect(&TokenKind::Comma, "','")?;
        let account = self.parse_account_path()?;
        self.expect(&TokenKind::RParen, "')'")?;
        let end = self.expect(&TokenKind::Semi, "';'")?;
        Ok(Stmt::Absorb { residue_name, residue_span, account, span: start.to(end) })
    }

    fn parse_money_expr(&mut self) -> PResult<MoneyExpr> {
        match &self.peek().kind {
            TokenKind::KwCredit => self.parse_credit(),
            TokenKind::Ident(_) => {
                let (name, span) = self.parse_ident()?;
                Ok(MoneyExpr::Var { name, span })
            }
            _ => Err(self.unexpected("'credit(...)' or a variable name")),
        }
    }

    fn parse_credit(&mut self) -> PResult<MoneyExpr> {
        let start = self.expect(&TokenKind::KwCredit, "'credit'")?;
        self.expect(&TokenKind::LParen, "'('")?;
        let account = self.parse_account_path()?;
        self.expect(&TokenKind::Comma, "','")?;
        let amount = self.parse_amount()?;
        let end = self.expect(&TokenKind::RParen, "')'")?;
        Ok(MoneyExpr::Credit { account, amount, span: start.to(end) })
    }

    fn parse_account_path(&mut self) -> PResult<AccountPath> {
        let mut segments = Vec::new();
        let first = self.parse_ident()?;
        let mut span = first.1;
        segments.push(first.0);
        while self.peek().kind == TokenKind::Colon {
            self.advance();
            let (seg, seg_span) = self.parse_ident()?;
            segments.push(seg);
            span = span.to(seg_span);
        }
        Ok(AccountPath { segments, span })
    }

    fn parse_ident(&mut self) -> PResult<(String, Span)> {
        match &self.peek().kind {
            TokenKind::Ident(s) => {
                let s = s.clone();
                let span = self.advance().span;
                Ok((s, span))
            }
            _ => Err(self.unexpected("an identifier")),
        }
    }

    fn parse_amount(&mut self) -> PResult<DecimalLiteral> {
        match &self.peek().kind {
            TokenKind::Decimal(text) => {
                let text = text.clone();
                let span = self.advance().span;
                Ok(DecimalLiteral { text, span })
            }
            _ => Err(self.unexpected("an amount")),
        }
    }
}

fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::KwTxn => "'txn'".to_string(),
        TokenKind::KwLet => "'let'".to_string(),
        TokenKind::KwDebit => "'debit'".to_string(),
        TokenKind::KwCredit => "'credit'".to_string(),
        TokenKind::KwCurrency => "'currency'".to_string(),
        TokenKind::KwAccount => "'account'".to_string(),
        TokenKind::KwScale => "'scale'".to_string(),
        TokenKind::KwRate => "'rate'".to_string(),
        TokenKind::KwFrom => "'from'".to_string(),
        TokenKind::KwTo => "'to'".to_string(),
        TokenKind::KwRound => "'round'".to_string(),
        TokenKind::KwDown => "'down'".to_string(),
        TokenKind::KwConvert => "'convert'".to_string(),
        TokenKind::KwAbsorb => "'absorb'".to_string(),
        TokenKind::Ident(s) => format!("identifier '{s}'"),
        TokenKind::Decimal(s) => format!("number '{s}'"),
        TokenKind::Str(s) => format!("string \"{s}\""),
        TokenKind::LParen => "'('".to_string(),
        TokenKind::RParen => "')'".to_string(),
        TokenKind::LBrace => "'{'".to_string(),
        TokenKind::RBrace => "'}'".to_string(),
        TokenKind::Comma => "','".to_string(),
        TokenKind::Colon => "':'".to_string(),
        TokenKind::Semi => "';'".to_string(),
        TokenKind::Eq => "'='".to_string(),
        TokenKind::Eof => "end of input".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::lex;

    fn parse_src(src: &str) -> (Option<Module>, Vec<Diagnostic>) {
        let (tokens, diags) = lex(src);
        assert!(diags.is_empty(), "lex failed: {diags:?}");
        parse(&tokens.unwrap())
    }

    #[test]
    fn parses_let_and_debit() {
        let (module, diags) = parse_src(
            r#"txn "coffee" { let m = credit(assets:cash, 45.00); debit(expenses:coffee, m); }"#,
        );
        assert!(diags.is_empty());
        let module = module.unwrap();
        assert_eq!(module.txns.len(), 1);
        let txn = &module.txns[0];
        assert_eq!(txn.name, "coffee");
        assert_eq!(txn.stmts.len(), 2);
        match &txn.stmts[0] {
            Stmt::Let { name, value: MoneyExpr::Credit { account, amount, .. }, .. } => {
                assert_eq!(name, "m");
                assert_eq!(account.to_string(), "assets:cash");
                assert_eq!(amount.text, "45.00");
            }
            _ => panic!("expected a let-binding of a credit"),
        }
        match &txn.stmts[1] {
            Stmt::Debit { account, value: MoneyExpr::Var { name, .. }, .. } => {
                assert_eq!(account.to_string(), "expenses:coffee");
                assert_eq!(name, "m");
            }
            _ => panic!("expected a debit of a variable"),
        }
    }

    #[test]
    fn credit_can_be_inlined_in_a_debit() {
        let (module, diags) =
            parse_src(r#"txn "coffee" { debit(expenses:coffee, credit(assets:cash, 45.00)); }"#);
        assert!(diags.is_empty());
        let module = module.unwrap();
        assert_eq!(module.txns[0].stmts.len(), 1);
        assert!(matches!(
            &module.txns[0].stmts[0],
            Stmt::Debit { value: MoneyExpr::Credit { .. }, .. }
        ));
    }

    #[test]
    fn parses_currency_and_account_decls() {
        let (module, diags) = parse_src(
            r#"
            currency ETB { scale = 2 }
            account assets:cash { currency = ETB }
            txn "coffee" { let m = credit(assets:cash, 45.00); debit(expenses:coffee, m); }
            "#,
        );
        assert!(diags.is_empty());
        let module = module.unwrap();
        assert_eq!(module.currencies.len(), 1);
        assert_eq!(module.currencies[0].name, "ETB");
        assert_eq!(module.currencies[0].scale, 2);
        assert_eq!(module.accounts.len(), 1);
        assert_eq!(module.accounts[0].path.to_string(), "assets:cash");
        assert_eq!(module.accounts[0].currency, "ETB");
    }

    #[test]
    fn parses_rate_decl() {
        let (module, diags) = parse_src(
            r#"rate usd_etb from USD to ETB = 57.20 round down;
               txn "t" { }"#,
        );
        assert!(diags.is_empty());
        let module = module.unwrap();
        assert_eq!(module.rates.len(), 1);
        let rate = &module.rates[0];
        assert_eq!(rate.name, "usd_etb");
        assert_eq!(rate.from, "USD");
        assert_eq!(rate.to, "ETB");
        assert_eq!(rate.value.text, "57.20");
    }

    #[test]
    fn parses_convert_and_absorb() {
        let (module, diags) = parse_src(
            r#"txn "fx" {
                let (m2, r) = convert(m, usd_etb);
                debit(expenses:x, m2);
                absorb(r, income:fx_rounding);
            }"#,
        );
        assert!(diags.is_empty());
        let module = module.unwrap();
        let stmts = &module.txns[0].stmts;
        assert_eq!(stmts.len(), 3);
        match &stmts[0] {
            Stmt::Convert { primary_name, residual_name, rate, .. } => {
                assert_eq!(primary_name, "m2");
                assert_eq!(residual_name, "r");
                assert_eq!(rate, "usd_etb");
            }
            _ => panic!("expected a convert statement"),
        }
        match &stmts[2] {
            Stmt::Absorb { residue_name, account, .. } => {
                assert_eq!(residue_name, "r");
                assert_eq!(account.to_string(), "income:fx_rounding");
            }
            _ => panic!("expected an absorb statement"),
        }
    }

    #[test]
    fn missing_brace_is_a_diagnostic_not_a_panic() {
        let (module, diags) =
            parse_src(r#"txn "coffee" { debit(expenses:coffee, credit(assets:cash, 45.00));"#);
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::ParseUnexpectedEof);
    }

    #[test]
    fn bare_literal_debit_argument_is_a_parse_error() {
        // Slice 0's grammar allowed `debit(acct, 45.00)` directly; Slice 1 requires a
        // MoneyExpr (a `credit(...)` or a variable) in that position (D-005).
        let (module, diags) = parse_src(r#"txn "coffee" { debit(expenses:coffee, 45.00); }"#);
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::ParseUnexpectedToken);
    }
}
