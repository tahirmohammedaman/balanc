//! `Vec<Token>` -> `ast::Module`, hand-written recursive descent (D-007).
//!
//! Grammar (Slice 4):
//!
//! ```text
//! Module        := Decl* Eof
//! Decl          := CurrencyDecl | AccountDecl | RateDecl | TxnDecl
//! CurrencyDecl  := "currency" Ident "{" "scale" "=" WholeNumber "}"
//! AccountDecl   := "account" AccountPath "{" "currency" "=" Ident ","
//!                    "kind" "=" Ident "," "normal" "=" ("debit" | "credit") "}"
//! RateDecl      := "rate" Ident "from" Ident "to" Ident "=" Decimal "round" "down" ";"
//! TxnDecl       := "txn" String "{" Stmt* "}"
//! Stmt          := LetStmt | PairStmt | DebitStmt | AbsorbStmt
//! LetStmt       := "let" Ident "=" MoneyExpr ";"
//! PairStmt      := "let" "(" Ident "," Ident ")" "=" (ConvertCall | SplitCall | SplitRatioCall) ";"
//! ConvertCall   := "convert" "(" MoneyExpr "," Ident ")"
//! SplitCall     := "split" "(" MoneyExpr "," Decimal ")"
//! SplitRatioCall:= "split_ratio" "(" MoneyExpr "," WholeNumber "," WholeNumber ")"
//! DebitStmt     := "debit" "(" AccountPath "," MoneyExpr ")" ";"
//! AbsorbStmt    := "absorb" "(" Ident "," AccountPath ")" ";"
//! MoneyExpr     := CreditExpr | MergeExpr | Var
//! CreditExpr    := "credit" "(" AccountPath "," Decimal ")"
//! MergeExpr     := "merge" "(" MoneyExpr "," MoneyExpr ")"
//! Var           := Ident
//! AccountPath   := Ident (":" Ident)*
//! ```
//!
//! `CurrencyDecl`, `AccountDecl`, `RateDecl`, and `TxnDecl` may appear in any order at
//! module level — nothing here requires currencies before the accounts/rates that use
//! them, or any of those before the transactions that reference them; `resolve` builds
//! all three tables before walking transaction bodies. `PairStmt`'s three call forms
//! are distinguished from `LetStmt`, and from each other, by lookahead only: `let`
//! followed by `(` is always a `PairStmt` (an ordinary `let`'s left-hand side is a
//! single `Ident`, never a parenthesized pair), and the keyword right after `=`
//! decides which of the three it is — no backtracking needed either way. `RateDecl`'s
//! `round down` is required syntax even though `down` is the only value that currently
//! type-checks (D-027).
//!
//! `MergeExpr` is the first place `MoneyExpr` nests (every other `MoneyExpr` position
//! takes an `AccountPath`/`Decimal`/`Ident`, never another `MoneyExpr`) — see
//! `Parser::parse_money_expr`'s depth cap, `MAX_MONEY_EXPR_DEPTH`.
//!
//! **Error recovery (Slice 6).** A malformed `Decl` or `Stmt` no longer aborts the
//! whole parse: `parse_module`'s loop and `parse_txn`'s statement loop each catch the
//! `Err` locally, record its diagnostic, and resynchronise —
//! `Parser::synchronize_decl` skips to the next `currency`/`account`/`rate`/`txn`
//! keyword (or `Eof`), `Parser::synchronize_stmt` skips to the next `;`, `}`, or
//! `let`/`debit`/`absorb` keyword — before continuing to parse whatever follows. This
//! is deliberately coarse: the malformed `Decl`/`Stmt` itself is discarded rather than
//! replaced with a placeholder AST node, since every diagnostic raised during recovery
//! already carries the real span of the token that triggered it (invariant 6 asks for
//! a real span on the *diagnostic*, not for the malformed node to survive into the
//! tree) and `parse` returns `None` for the module whenever any diagnostics were
//! recorded, so `resolve`/`typeck`/`eval` never see a partial `Decl`/`Stmt` either way
//! (D-038). The payoff is purely diagnostic: a file with several independent syntax
//! errors reports all of them in one run, sorted by span, instead of just the first.

pub mod ast;

use crate::diag::{Code, Diagnostic};
use crate::lex::{Token, TokenKind};
use crate::span::Span;
use ast::{
    AccountDecl, AccountPath, CurrencyDecl, DecimalLiteral, Module, MoneyExpr, NormalBalance,
    RateDecl, Stmt, TxnDecl,
};

/// How many `MoneyExpr`s deep a `merge(...)` chain may nest before parsing gives up
/// with a diagnostic instead of recursing further. Chosen generously relative to any
/// real ledger expression (deeper than this is already an absurd program) while
/// staying well inside a safe recursive-descent stack budget.
const MAX_MONEY_EXPR_DEPTH: u32 = 256;

pub fn parse(tokens: &[Token]) -> (Option<Module>, Vec<Diagnostic>) {
    let mut p = Parser { tokens, pos: 0, diags: Vec::new() };
    let module = p.parse_module();
    if p.diags.is_empty() {
        (Some(module), Vec::new())
    } else {
        (None, p.diags)
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    /// Diagnostics recorded during recovery (see the module doc comment). A `PResult`
    /// error returned all the way out of `parse_module` is a bug, not a user error —
    /// `parse_module`'s own loop and `parse_txn`'s statement loop are the only two
    /// places an `Err` is caught and folded in here instead of propagated with `?`.
    diags: Vec<Diagnostic>,
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

    fn parse_module(&mut self) -> Module {
        let mut currencies = Vec::new();
        let mut accounts = Vec::new();
        let mut rates = Vec::new();
        let mut txns = Vec::new();
        while self.peek().kind != TokenKind::Eof {
            let result = match &self.peek().kind {
                TokenKind::KwCurrency => self.parse_currency_decl().map(|d| currencies.push(d)),
                TokenKind::KwAccount => self.parse_account_decl().map(|d| accounts.push(d)),
                TokenKind::KwRate => self.parse_rate_decl().map(|d| rates.push(d)),
                TokenKind::KwTxn => self.parse_txn().map(|d| txns.push(d)),
                _ => Err(self.unexpected("'currency', 'account', 'rate', or 'txn'")),
            };
            if let Err(diag) = result {
                self.diags.push(diag);
                self.synchronize_decl();
            }
        }
        Module { currencies, accounts, rates, txns }
    }

    /// Skips tokens until the next declaration keyword (or `Eof`), so one malformed
    /// `currency`/`account`/`rate`/`txn` declaration doesn't stop the rest of the
    /// module from being parsed.
    fn synchronize_decl(&mut self) {
        while !matches!(
            self.peek().kind,
            TokenKind::KwCurrency
                | TokenKind::KwAccount
                | TokenKind::KwRate
                | TokenKind::KwTxn
                | TokenKind::Eof
        ) {
            self.advance();
        }
    }

    /// Skips tokens until (and including) the next `;`, or up to the next `}` or
    /// statement keyword (`let`/`debit`/`absorb`) without consuming it, so one
    /// malformed statement doesn't stop the rest of the transaction body from being
    /// parsed. A `;` is the common case and is consumed since it already terminates
    /// the broken statement; `}`/a statement keyword is left for the caller (the
    /// txn-body loop, or `parse_module` if this statement's `;` never comes) to see
    /// and act on itself.
    fn synchronize_stmt(&mut self) {
        loop {
            match &self.peek().kind {
                TokenKind::Semi => {
                    self.advance();
                    return;
                }
                TokenKind::RBrace
                | TokenKind::Eof
                | TokenKind::KwLet
                | TokenKind::KwDebit
                | TokenKind::KwAbsorb => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn parse_currency_decl(&mut self) -> PResult<CurrencyDecl> {
        let start = self.expect(&TokenKind::KwCurrency, "'currency'")?;
        let (name, name_span) = self.parse_ident()?;
        self.expect(&TokenKind::LBrace, "'{'")?;
        self.expect(&TokenKind::KwScale, "'scale'")?;
        self.expect(&TokenKind::Eq, "'='")?;
        let (scale, scale_span) = self.parse_whole_number("a whole number")?;
        let end = self.expect(&TokenKind::RBrace, "'}'")?;
        Ok(CurrencyDecl { name, name_span, scale, scale_span, span: start.to(end) })
    }

    /// Shared by `CurrencyDecl`'s `scale` and `SplitRatioCall`'s two weights (D-035) —
    /// both want a bare non-negative integer, not a `Decimal` (which allows a `.`).
    fn parse_whole_number(&mut self, expected: &str) -> PResult<(u32, Span)> {
        match &self.peek().kind {
            TokenKind::Decimal(text) if !text.contains('.') => {
                let text = text.clone();
                let span = self.advance().span;
                let value = text.parse().map_err(|_| {
                    Diagnostic::new(Code::ParseUnexpectedToken, "number is too large", span)
                })?;
                Ok((value, span))
            }
            _ => Err(self.unexpected(expected)),
        }
    }

    fn parse_account_decl(&mut self) -> PResult<AccountDecl> {
        let start = self.expect(&TokenKind::KwAccount, "'account'")?;
        let path = self.parse_account_path()?;
        self.expect(&TokenKind::LBrace, "'{'")?;
        self.expect(&TokenKind::KwCurrency, "'currency'")?;
        self.expect(&TokenKind::Eq, "'='")?;
        let (currency, currency_span) = self.parse_ident()?;
        self.expect(&TokenKind::Comma, "','")?;
        self.expect(&TokenKind::KwKind, "'kind'")?;
        self.expect(&TokenKind::Eq, "'='")?;
        let (kind, kind_span) = self.parse_ident()?;
        self.expect(&TokenKind::Comma, "','")?;
        self.expect(&TokenKind::KwNormal, "'normal'")?;
        self.expect(&TokenKind::Eq, "'='")?;
        let (normal, normal_span) = self.parse_normal_balance()?;
        let end = self.expect(&TokenKind::RBrace, "'}'")?;
        Ok(AccountDecl {
            path,
            currency,
            currency_span,
            kind,
            kind_span,
            normal,
            normal_span,
            span: start.to(end),
        })
    }

    fn parse_normal_balance(&mut self) -> PResult<(NormalBalance, Span)> {
        match &self.peek().kind {
            TokenKind::KwDebit => Ok((NormalBalance::Debit, self.advance().span)),
            TokenKind::KwCredit => Ok((NormalBalance::Credit, self.advance().span)),
            _ => Err(self.unexpected("'debit' or 'credit'")),
        }
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
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(diag) => {
                    self.diags.push(diag);
                    self.synchronize_stmt();
                }
            }
        }
        let end = self.expect(&TokenKind::RBrace, "'}'")?;
        Ok(TxnDecl { name, stmts, span: start.to(end) })
    }

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        match &self.peek().kind {
            TokenKind::KwLet => self.parse_let_or_pair(),
            TokenKind::KwDebit => self.parse_debit(),
            TokenKind::KwAbsorb => self.parse_absorb(),
            _ => Err(self.unexpected("'let', 'debit', or 'absorb'")),
        }
    }

    /// `let` followed by `(` is always one of `PairStmt`'s three forms — an ordinary
    /// `let`'s left-hand side is a single `Ident`, never a parenthesized pair — so one
    /// token of lookahead after `let` disambiguates `LetStmt` from `PairStmt` without
    /// backtracking; which of the three `PairStmt` forms it is then comes from the
    /// keyword right after `=`, one more token of lookahead.
    fn parse_let_or_pair(&mut self) -> PResult<Stmt> {
        let start = self.expect(&TokenKind::KwLet, "'let'")?;
        if self.peek().kind == TokenKind::LParen {
            self.parse_pair_stmt(start)
        } else {
            let (name, name_span) = self.parse_ident()?;
            self.expect(&TokenKind::Eq, "'='")?;
            let value = self.parse_money_expr(0)?;
            let end = self.expect(&TokenKind::Semi, "';'")?;
            Ok(Stmt::Let { name, name_span, value, span: start.to(end) })
        }
    }

    fn parse_pair_stmt(&mut self, start: Span) -> PResult<Stmt> {
        self.expect(&TokenKind::LParen, "'('")?;
        let (a_name, a_span) = self.parse_ident()?;
        self.expect(&TokenKind::Comma, "','")?;
        let (b_name, b_span) = self.parse_ident()?;
        self.expect(&TokenKind::RParen, "')'")?;
        self.expect(&TokenKind::Eq, "'='")?;
        match &self.peek().kind {
            TokenKind::KwConvert => self.parse_convert(start, a_name, a_span, b_name, b_span),
            TokenKind::KwSplit => self.parse_split(start, a_name, a_span, b_name, b_span),
            TokenKind::KwSplitRatio => {
                self.parse_split_ratio(start, a_name, a_span, b_name, b_span)
            }
            _ => Err(self.unexpected("'convert', 'split', or 'split_ratio'")),
        }
    }

    fn parse_convert(
        &mut self,
        start: Span,
        primary_name: String,
        primary_span: Span,
        residual_name: String,
        residual_span: Span,
    ) -> PResult<Stmt> {
        self.expect(&TokenKind::KwConvert, "'convert'")?;
        self.expect(&TokenKind::LParen, "'('")?;
        let money = self.parse_money_expr(0)?;
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

    fn parse_split(
        &mut self,
        start: Span,
        a_name: String,
        a_span: Span,
        b_name: String,
        b_span: Span,
    ) -> PResult<Stmt> {
        self.expect(&TokenKind::KwSplit, "'split'")?;
        self.expect(&TokenKind::LParen, "'('")?;
        let money = self.parse_money_expr(0)?;
        self.expect(&TokenKind::Comma, "','")?;
        let amount = self.parse_amount()?;
        self.expect(&TokenKind::RParen, "')'")?;
        let end = self.expect(&TokenKind::Semi, "';'")?;
        Ok(Stmt::Split { a_name, a_span, b_name, b_span, money, amount, span: start.to(end) })
    }

    fn parse_split_ratio(
        &mut self,
        start: Span,
        a_name: String,
        a_span: Span,
        b_name: String,
        b_span: Span,
    ) -> PResult<Stmt> {
        self.expect(&TokenKind::KwSplitRatio, "'split_ratio'")?;
        self.expect(&TokenKind::LParen, "'('")?;
        let money = self.parse_money_expr(0)?;
        self.expect(&TokenKind::Comma, "','")?;
        let (a_weight, a_weight_span) = self.parse_whole_number("a whole-number ratio weight")?;
        self.expect(&TokenKind::Comma, "','")?;
        let (b_weight, b_weight_span) = self.parse_whole_number("a whole-number ratio weight")?;
        self.expect(&TokenKind::RParen, "')'")?;
        let end = self.expect(&TokenKind::Semi, "';'")?;
        Ok(Stmt::SplitRatio {
            a_name,
            a_span,
            b_name,
            b_span,
            money,
            a_weight,
            a_weight_span,
            b_weight,
            b_weight_span,
            span: start.to(end),
        })
    }

    fn parse_debit(&mut self) -> PResult<Stmt> {
        let start = self.expect(&TokenKind::KwDebit, "'debit'")?;
        self.expect(&TokenKind::LParen, "'('")?;
        let account = self.parse_account_path()?;
        self.expect(&TokenKind::Comma, "','")?;
        let value = self.parse_money_expr(0)?;
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

    /// `depth` counts how many `MergeExpr`s deep this call is nested — every other
    /// `MoneyExpr` production is a leaf, so only `parse_merge` ever recurses back into
    /// this function with `depth + 1`.
    fn parse_money_expr(&mut self, depth: u32) -> PResult<MoneyExpr> {
        if depth >= MAX_MONEY_EXPR_DEPTH {
            return Err(Diagnostic::new(
                Code::ExprTooDeep,
                format!("expression nested more than {MAX_MONEY_EXPR_DEPTH} levels deep"),
                self.peek().span,
            ));
        }
        match &self.peek().kind {
            TokenKind::KwCredit => self.parse_credit(),
            TokenKind::KwMerge => self.parse_merge(depth),
            TokenKind::Ident(_) => {
                let (name, span) = self.parse_ident()?;
                Ok(MoneyExpr::Var { name, span })
            }
            _ => Err(self.unexpected("'credit(...)', 'merge(...)', or a variable name")),
        }
    }

    fn parse_merge(&mut self, depth: u32) -> PResult<MoneyExpr> {
        let start = self.expect(&TokenKind::KwMerge, "'merge'")?;
        self.expect(&TokenKind::LParen, "'('")?;
        let a = self.parse_money_expr(depth + 1)?;
        self.expect(&TokenKind::Comma, "','")?;
        let b = self.parse_money_expr(depth + 1)?;
        let end = self.expect(&TokenKind::RParen, "')'")?;
        Ok(MoneyExpr::Merge { a: Box::new(a), b: Box::new(b), span: start.to(end) })
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
        TokenKind::KwKind => "'kind'".to_string(),
        TokenKind::KwNormal => "'normal'".to_string(),
        TokenKind::KwRate => "'rate'".to_string(),
        TokenKind::KwFrom => "'from'".to_string(),
        TokenKind::KwTo => "'to'".to_string(),
        TokenKind::KwRound => "'round'".to_string(),
        TokenKind::KwDown => "'down'".to_string(),
        TokenKind::KwConvert => "'convert'".to_string(),
        TokenKind::KwAbsorb => "'absorb'".to_string(),
        TokenKind::KwSplit => "'split'".to_string(),
        TokenKind::KwSplitRatio => "'split_ratio'".to_string(),
        TokenKind::KwMerge => "'merge'".to_string(),
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
            account assets:cash { currency = ETB, kind = asset, normal = debit }
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
        assert_eq!(module.accounts[0].kind, "asset");
        assert_eq!(module.accounts[0].normal, NormalBalance::Debit);
    }

    #[test]
    fn parses_credit_normal_balance() {
        let (module, diags) = parse_src(
            r#"currency ETB { scale = 2 }
               account equity:owner_capital { currency = ETB, kind = equity, normal = credit }
               txn "t" { }"#,
        );
        assert!(diags.is_empty());
        let module = module.unwrap();
        assert_eq!(module.accounts[0].kind, "equity");
        assert_eq!(module.accounts[0].normal, NormalBalance::Credit);
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

    #[test]
    fn parses_split() {
        let (module, diags) = parse_src(
            r#"txn "t" {
                let (a, b) = split(m, 20.00);
                debit(expenses:a, a);
                debit(expenses:b, b);
            }"#,
        );
        assert!(diags.is_empty());
        let module = module.unwrap();
        match &module.txns[0].stmts[0] {
            Stmt::Split { a_name, b_name, amount, .. } => {
                assert_eq!(a_name, "a");
                assert_eq!(b_name, "b");
                assert_eq!(amount.text, "20.00");
            }
            _ => panic!("expected a split statement"),
        }
    }

    #[test]
    fn parses_split_ratio() {
        let (module, diags) = parse_src(
            r#"txn "t" {
                let (a, b) = split_ratio(m, 1, 2);
                debit(expenses:a, a);
                debit(expenses:b, b);
            }"#,
        );
        assert!(diags.is_empty());
        let module = module.unwrap();
        match &module.txns[0].stmts[0] {
            Stmt::SplitRatio { a_name, b_name, a_weight, b_weight, .. } => {
                assert_eq!(a_name, "a");
                assert_eq!(b_name, "b");
                assert_eq!(*a_weight, 1);
                assert_eq!(*b_weight, 2);
            }
            _ => panic!("expected a split_ratio statement"),
        }
    }

    #[test]
    fn split_ratio_rejects_a_fractional_weight() {
        let (module, diags) =
            parse_src(r#"txn "t" { let (a, b) = split_ratio(m, 1.5, 2); }"#);
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::ParseUnexpectedToken);
    }

    #[test]
    fn parses_merge_including_nested() {
        let (module, diags) = parse_src(
            r#"txn "t" { debit(expenses:coffee, merge(credit(assets:cash, 5.00), merge(a, b))); }"#,
        );
        assert!(diags.is_empty());
        let module = module.unwrap();
        assert!(matches!(
            &module.txns[0].stmts[0],
            Stmt::Debit { value: MoneyExpr::Merge { .. }, .. }
        ));
    }

    #[test]
    fn deeply_nested_merge_is_rejected_as_too_deep() {
        let mut src = String::from(r#"txn "t" { debit(expenses:coffee, "#);
        for _ in 0..(MAX_MONEY_EXPR_DEPTH + 1) {
            src.push_str("merge(a, ");
        }
        src.push('a');
        for _ in 0..(MAX_MONEY_EXPR_DEPTH + 1) {
            src.push(')');
        }
        src.push_str("); }");
        let (module, diags) = parse_src(&src);
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::ExprTooDeep);
    }

    #[test]
    fn merge_within_the_depth_cap_still_parses() {
        let mut src = String::from(r#"txn "t" { debit(expenses:coffee, "#);
        for _ in 0..(MAX_MONEY_EXPR_DEPTH - 1) {
            src.push_str("merge(a, ");
        }
        src.push('a');
        for _ in 0..(MAX_MONEY_EXPR_DEPTH - 1) {
            src.push(')');
        }
        src.push_str("); }");
        let (module, diags) = parse_src(&src);
        assert!(diags.is_empty(), "{diags:?}");
        assert!(module.is_some());
    }

    #[test]
    fn three_independent_syntax_errors_are_all_reported_sorted_by_span() {
        // Each error sits in a different declaration and doesn't touch the others: a
        // missing '=' in the account decl, a bare literal where a MoneyExpr is
        // required, and a missing ',' in a debit's argument list.
        let (module, mut diags) = parse_src(
            r#"
            currency ETB { scale = 2 }
            account assets:cash { currency ETB, kind = asset, normal = debit }
            txn "t" {
                let m = 5;
                debit(expenses:coffee m);
            }
            "#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 3, "{diags:?}");
        crate::diag::sort_by_span(&mut diags);
        assert_eq!(diags[0].code, Code::ParseUnexpectedToken); // "currency ETB" missing '='
        assert_eq!(diags[1].code, Code::ParseUnexpectedToken); // "let m = 5" not a MoneyExpr
        assert_eq!(diags[2].code, Code::ParseUnexpectedToken); // "expenses:coffee m" missing ','
        assert!(diags[0].span.lo < diags[1].span.lo);
        assert!(diags[1].span.lo < diags[2].span.lo);
    }

    #[test]
    fn recovery_after_a_bad_declaration_still_parses_the_next_one() {
        // The malformed `account` decl (missing '=') is dropped entirely by recovery,
        // but the well-formed `txn` after it still parses.
        let (module, diags) = parse_src(
            r#"
            account assets:cash { currency ETB, kind = asset, normal = debit }
            txn "t" { }
            "#,
        );
        assert!(module.is_none());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::ParseUnexpectedToken);
    }
}
