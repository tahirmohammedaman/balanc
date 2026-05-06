//! `&str` -> `Vec<Token>`. Tokens carry spans (invariant 6); nothing downstream of the
//! lexer touches source text directly except `diag`, which resolves a span back to
//! line/column for rendering.
//!
//! Money amounts have a fixed scale of 2 for Slice 0 (per-currency scale lands in
//! Slice 2), so a decimal literal with more than two fraction digits is a lex error.
//! There is no `-` token in numeric-literal position, so a negative literal cannot be
//! written at all (D-014) — this is enforced by omission, not by a dedicated check.

use crate::diag::{Code, Diagnostic};
use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    KwTxn,
    KwLet,
    KwDebit,
    KwCredit,
    Ident(String),
    /// Raw literal text, e.g. `"45"` or `"45.00"`; lowering to `Amount` happens in the
    /// parser, which is where a per-syntax-position error (e.g. wrong arg count)
    /// would also be reported.
    Decimal(String),
    /// Contents of a string literal, with surrounding quotes stripped.
    Str(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Semi,
    Eq,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Lexes `source`. Returns `None` for the token stream on the first fatal error
/// (unterminated string, unexpected character, malformed literal) along with the
/// diagnostic explaining why — Slice 0 has no error recovery, so there is never more
/// than one lex diagnostic per run.
pub fn lex(source: &str) -> (Option<Vec<Token>>, Vec<Diagnostic>) {
    let bytes = source.as_bytes();
    let mut pos: usize = 0;
    let mut tokens = Vec::new();

    macro_rules! fatal {
        ($code:expr, $msg:expr, $span:expr) => {
            return (None, vec![Diagnostic::new($code, $msg, $span)])
        };
    }

    while pos < bytes.len() {
        let ch = bytes[pos] as char;

        if ch.is_ascii_whitespace() {
            pos += 1;
            continue;
        }

        if ch == '/' && bytes.get(pos + 1) == Some(&b'/') {
            while pos < bytes.len() && bytes[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }

        let start = pos;

        match ch {
            '(' => {
                tokens.push(tok(TokenKind::LParen, start, start + 1));
                pos += 1;
            }
            ')' => {
                tokens.push(tok(TokenKind::RParen, start, start + 1));
                pos += 1;
            }
            '{' => {
                tokens.push(tok(TokenKind::LBrace, start, start + 1));
                pos += 1;
            }
            '}' => {
                tokens.push(tok(TokenKind::RBrace, start, start + 1));
                pos += 1;
            }
            ',' => {
                tokens.push(tok(TokenKind::Comma, start, start + 1));
                pos += 1;
            }
            ':' => {
                tokens.push(tok(TokenKind::Colon, start, start + 1));
                pos += 1;
            }
            ';' => {
                tokens.push(tok(TokenKind::Semi, start, start + 1));
                pos += 1;
            }
            '=' => {
                tokens.push(tok(TokenKind::Eq, start, start + 1));
                pos += 1;
            }
            '"' => {
                pos += 1;
                let content_start = pos;
                while pos < bytes.len() && bytes[pos] != b'"' && bytes[pos] != b'\n' {
                    pos += 1;
                }
                if pos >= bytes.len() || bytes[pos] != b'"' {
                    fatal!(
                        Code::LexUnexpectedChar,
                        "unterminated string literal",
                        Span::new(start as u32, pos as u32)
                    );
                }
                let content = &source[content_start..pos];
                pos += 1; // closing quote
                tokens.push(tok(TokenKind::Str(content.to_string()), start, pos));
            }
            c if c.is_ascii_digit() => {
                while pos < bytes.len() && (bytes[pos] as char).is_ascii_digit() {
                    pos += 1;
                }
                let mut frac_digits = 0usize;
                let mut frac_start = pos;
                if pos < bytes.len() && bytes[pos] == b'.' {
                    pos += 1;
                    frac_start = pos;
                    while pos < bytes.len() && (bytes[pos] as char).is_ascii_digit() {
                        pos += 1;
                        frac_digits += 1;
                    }
                }
                if frac_digits > 2 {
                    fatal!(
                        Code::LexTooManyFractionDigits,
                        "amounts have at most 2 fractional digits",
                        Span::new(frac_start as u32, pos as u32)
                    );
                }
                let text = &source[start..pos];
                tokens.push(tok(TokenKind::Decimal(text.to_string()), start, pos));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                while pos < bytes.len() {
                    let c = bytes[pos] as char;
                    if c.is_ascii_alphanumeric() || c == '_' {
                        pos += 1;
                    } else {
                        break;
                    }
                }
                let text = &source[start..pos];
                let kind = match text {
                    "txn" => TokenKind::KwTxn,
                    "let" => TokenKind::KwLet,
                    "debit" => TokenKind::KwDebit,
                    "credit" => TokenKind::KwCredit,
                    _ => TokenKind::Ident(text.to_string()),
                };
                tokens.push(tok(kind, start, pos));
            }
            _ => {
                fatal!(
                    Code::LexUnexpectedChar,
                    format!("unexpected character '{ch}'"),
                    Span::new(start as u32, (start + ch.len_utf8()) as u32)
                );
            }
        }
    }

    tokens.push(tok(TokenKind::Eof, bytes.len(), bytes.len()));
    (Some(tokens), Vec::new())
}

fn tok(kind: TokenKind, lo: usize, hi: usize) -> Token {
    Token { kind, span: Span::new(lo as u32, hi as u32) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_round_trip_to_source_text() {
        let src = r#"txn "coffee" { let m = credit(assets:cash, 45.00); debit(expenses:coffee, m); }"#;
        let (tokens, diags) = lex(src);
        assert!(diags.is_empty());
        let tokens = tokens.unwrap();
        for t in &tokens {
            if t.kind == TokenKind::Eof {
                continue;
            }
            let slice = &src[t.span.lo as usize..t.span.hi as usize];
            match &t.kind {
                TokenKind::KwTxn => assert_eq!(slice, "txn"),
                TokenKind::KwDebit => assert_eq!(slice, "debit"),
                TokenKind::Ident(s) => assert_eq!(slice, s),
                TokenKind::Decimal(s) => assert_eq!(slice, s),
                TokenKind::Str(s) => assert_eq!(slice, format!("\"{s}\"")),
                _ => {}
            }
        }
    }

    #[test]
    fn keywords_recognized() {
        let (tokens, _) = lex("txn let debit credit");
        let kinds: Vec<_> = tokens.unwrap().into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwTxn,
                TokenKind::KwLet,
                TokenKind::KwDebit,
                TokenKind::KwCredit,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn too_many_fraction_digits_is_fatal() {
        let (tokens, diags) = lex("45.123");
        assert!(tokens.is_none());
        assert_eq!(diags[0].code, Code::LexTooManyFractionDigits);
    }

    #[test]
    fn unexpected_char_is_fatal() {
        let (tokens, diags) = lex("45.00 @ 3");
        assert!(tokens.is_none());
        assert_eq!(diags[0].code, Code::LexUnexpectedChar);
    }

    #[test]
    fn no_minus_token_negative_literal_is_unlexable() {
        // No '-' token exists at all: this lexes as unexpected-char, which is how
        // D-014 ("negative literal is a parse-time error") falls out of the grammar.
        let (tokens, diags) = lex("-45.00");
        assert!(tokens.is_none());
        assert_eq!(diags[0].code, Code::LexUnexpectedChar);
    }
}
