//! `Vec<Token>` -> `ast::Module`, hand-written recursive descent (D-007).
//!
//! Grammar (Slice 0, expression style per D-012):
//!
//! ```text
//! Module      := TxnDecl* Eof
//! TxnDecl     := "txn" String "{" Leg* "}"
//! Leg         := ("debit" | "credit") "(" AccountPath "," Decimal ")" ";"
//! AccountPath := Ident (":" Ident)*
//! ```
//!
//! No error recovery yet (Slice 6): the first unexpected token stops parsing and
//! returns a single diagnostic, matching the lexer's fatal-error behaviour.

pub mod ast;

use crate::amount::Amount;
use crate::diag::{Code, Diagnostic};
use crate::lex::{Token, TokenKind};
use crate::span::Span;
use ast::{AccountPath, Leg, LegKind, Module, TxnDecl};

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
        let mut txns = Vec::new();
        while self.peek().kind != TokenKind::Eof {
            txns.push(self.parse_txn()?);
        }
        Ok(Module { txns })
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
        let mut legs = Vec::new();
        while self.peek().kind != TokenKind::RBrace {
            if self.peek().kind == TokenKind::Eof {
                return Err(self.unexpected("'}'"));
            }
            legs.push(self.parse_leg()?);
        }
        let end = self.expect(&TokenKind::RBrace, "'}'")?;
        Ok(TxnDecl { name, legs, span: start.to(end) })
    }

    fn parse_leg(&mut self) -> PResult<Leg> {
        let (kind, start) = match &self.peek().kind {
            TokenKind::KwDebit => (LegKind::Debit, self.advance().span),
            TokenKind::KwCredit => (LegKind::Credit, self.advance().span),
            _ => return Err(self.unexpected("'debit' or 'credit'")),
        };
        self.expect(&TokenKind::LParen, "'('")?;
        let account = self.parse_account_path()?;
        self.expect(&TokenKind::Comma, "','")?;
        let amount = self.parse_amount()?;
        self.expect(&TokenKind::RParen, "')'")?;
        let end = self.expect(&TokenKind::Semi, "';'")?;
        Ok(Leg { kind, account, amount, span: start.to(end) })
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
            _ => Err(self.unexpected("an account name")),
        }
    }

    fn parse_amount(&mut self) -> PResult<Amount> {
        match &self.peek().kind {
            TokenKind::Decimal(text) => {
                let amount = amount_from_literal(text);
                self.advance();
                Ok(amount)
            }
            _ => Err(self.unexpected("an amount")),
        }
    }
}

fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::KwTxn => "'txn'".to_string(),
        TokenKind::KwDebit => "'debit'".to_string(),
        TokenKind::KwCredit => "'credit'".to_string(),
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
        TokenKind::Eof => "end of input".to_string(),
    }
}

/// Lowers literal text the lexer already validated (digits, optional '.', at most 2
/// fraction digits) into cents. Not a `TryFrom` because a malformed literal can never
/// reach here — the lexer rejects it first.
fn amount_from_literal(text: &str) -> Amount {
    let (whole, frac) = match text.split_once('.') {
        Some((w, f)) => (w, f),
        None => (text, ""),
    };
    let whole: i64 = whole.parse().expect("internal error: lexer produced a malformed decimal");
    let mut frac_cents: i64 = frac.parse().unwrap_or(0);
    if frac.len() == 1 {
        frac_cents *= 10;
    }
    Amount::from_cents(whole * 100 + frac_cents)
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
    fn parses_two_leg_transaction() {
        let (module, diags) = parse_src(
            r#"txn "coffee" { debit(expenses:coffee, 45.00); credit(assets:cash, 45.00); }"#,
        );
        assert!(diags.is_empty());
        let module = module.unwrap();
        assert_eq!(module.txns.len(), 1);
        let txn = &module.txns[0];
        assert_eq!(txn.name, "coffee");
        assert_eq!(txn.legs.len(), 2);
        assert_eq!(txn.legs[0].kind, LegKind::Debit);
        assert_eq!(txn.legs[0].account.to_string(), "expenses:coffee");
        assert_eq!(txn.legs[0].amount.cents(), 4500);
    }

    #[test]
    fn amount_literal_padding() {
        assert_eq!(amount_from_literal("45").cents(), 4500);
        assert_eq!(amount_from_literal("45.5").cents(), 4550);
        assert_eq!(amount_from_literal("45.00").cents(), 4500);
        assert_eq!(amount_from_literal("0.01").cents(), 1);
    }

    #[test]
    fn missing_brace_is_a_diagnostic_not_a_panic() {
        let (module, diags) = parse_src(r#"txn "coffee" { debit(expenses:coffee, 45.00);"#);
        assert!(module.is_none());
        assert_eq!(diags[0].code, Code::ParseUnexpectedEof);
    }
}
