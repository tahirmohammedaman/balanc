//! User-facing diagnostics. Every `Diagnostic` carries a stable `Code` and a real
//! `Span` (invariant 6) — never a synthetic or zero span.
//!
//! This is deliberately plain for Slice 0: rich rendering (source line, caret,
//! secondary labels) is Slice 6's job. For now a diagnostic prints as
//! `file:line:col: error[CODE]: message`.

pub mod codes;

use crate::span::{SourceFile, Span};
pub use codes::Code;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: Code,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn new(code: Code, message: impl Into<String>, span: Span) -> Self {
        Diagnostic { code, message: message.into(), span }
    }

    pub fn render(&self, file: &SourceFile) -> String {
        let (line, col) = file.line_col(self.span.lo);
        format!("{}:{}:{}: error[{}]: {}", file.name, line, col, self.code, self.message)
    }
}

/// Sort diagnostics by span so output order is deterministic regardless of which
/// stage or traversal order produced them.
pub fn sort_by_span(diags: &mut [Diagnostic]) {
    diags.sort_by_key(|d| (d.span.lo, d.span.hi));
}
