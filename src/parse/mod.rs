//! `Vec<Token>` -> `ast::Module`, hand-written recursive descent (D-007).
//!
//! Grammar (Slice 2):
//!
//! ```text
//! Module        := Decl* Eof
//! Decl          := CurrencyDecl | AccountDecl | TxnDecl
//! CurrencyDecl  := "currency" Ident "{" "scale" "=" Decimal "}"
//! AccountDecl   := "account" AccountPath "{" "currency" "=" Ident "}"
//! TxnDecl       := "txn" String "{" Stmt* "}"
//! Stmt          := LetStmt | DebitStmt
//! LetStmt       := "let" Ident "=" MoneyExpr ";"
//! DebitStmt     := "debit" "(" AccountPath "," MoneyExpr ")" ";"
//! MoneyExpr     := CreditExpr | Var
//! CreditExpr    := "credit" "(" AccountPath "," Decimal ")"
//! Var           := Ident
//! AccountPath   := Ident (":" Ident)*
//! ```
//!
//! `CurrencyDecl`, `AccountDecl`, and `TxnDecl` may appear in any order at module
//! level — nothing here requires currencies before the accounts that use them, or
//! either before the transactions that reference them; `resolve` builds both tables
//! before walking transaction bodies.
//!
//! No error recovery yet (Slice 6): the first unexpected token stops parsing and
//! returns a single diagnostic, matching the lexer's fatal-error behaviour.

pub mod ast;

use crate::diag::{Code, Diagnostic};
use crate::lex::{Token, TokenKind};
use crate::span::Span;
use ast::{AccountDecl, AccountPath, CurrencyDecl, DecimalLiteral, Module, MoneyExpr, Stmt, TxnDecl};

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
        let mut txns = Vec::new();
        while self.peek().kind != TokenKind::Eof {
            match &self.peek().kind {
                TokenKind::KwCurrency => currencies.push(self.parse_currency_decl()?),
                TokenKind::KwAccount => accounts.push(self.parse_account_decl()?),
                TokenKind::KwTxn => txns.push(self.parse_txn()?),
                _ => return Err(self.unexpected("'currency', 'account', or 'txn'")),
            }
        }
        Ok(Module { currencies, accounts, txns })
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
            TokenKind::KwLet => self.parse_let(),
            TokenKind::KwDebit => self.parse_debit(),
            _ => Err(self.unexpected("'let' or 'debit'")),
        }
    }

    fn parse_let(&mut self) -> PResult<Stmt> {
        let start = self.expect(&TokenKind::KwLet, "'let'")?;
        let (name, name_span) = self.parse_ident()?;
        self.expect(&TokenKind::Eq, "'='")?;
        let value = self.parse_money_expr()?;
        let end = self.expect(&TokenKind::Semi, "';'")?;
        Ok(Stmt::Let { name, name_span, value, span: start.to(end) })
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
