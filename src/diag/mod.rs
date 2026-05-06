//! User-facing diagnostics. Every `Diagnostic` carries a stable `Code` and a real
//! `Span` (invariant 6) — never a synthetic or zero span.
//!
//! This is deliberately plain through Slice 5: rich rendering (source line, caret) is
//! Slice 6's job. A diagnostic prints as `file:line:col: error[CODE]: message`, with
//! one `file:line:col: note: label` line per secondary span — enough to show, e.g.,
//! both use-sites of an `E_REUSED` value without pulling span-to-line/col resolution
//! out of this module (only `diag` does that; every other stage treats a `Span` as an
//! opaque range).

pub mod codes;

use crate::span::{SourceFile, Span};
pub use codes::Code;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: Code,
    pub message: String,
    pub span: Span,
    /// Extra `(label, span)` pairs — e.g. `("first consumed here", span)` — rendered
    /// as follow-up notes after the primary message.
    pub secondary: Vec<(String, Span)>,
}

impl Diagnostic {
    pub fn new(code: Code, message: impl Into<String>, span: Span) -> Self {
        Diagnostic { code, message: message.into(), span, secondary: Vec::new() }
    }

    pub fn with_secondary(mut self, label: impl Into<String>, span: Span) -> Self {
        self.secondary.push((label.into(), span));
        self
    }

    pub fn render(&self, file: &SourceFile) -> String {
        let (line, col) = file.line_col(self.span.lo);
        let mut out =
            format!("{}:{}:{}: error[{}]: {}", file.name, line, col, self.code, self.message);
        for (label, span) in &self.secondary {
            let (line, col) = file.line_col(span.lo);
            out.push('\n');
            out.push_str(&format!("{}:{}:{}: note: {}", file.name, line, col, label));
        }
        out
    }
}

/// Sort diagnostics by span so output order is deterministic regardless of which
/// stage or traversal order produced them.
pub fn sort_by_span(diags: &mut [Diagnostic]) {
    diags.sort_by_key(|d| (d.span.lo, d.span.hi));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::SourceFile;

    #[test]
    fn renders_primary_line_only_with_no_secondary_spans() {
        let file = SourceFile::new("t.bal".to_string(), "abc".to_string());
        let d = Diagnostic::new(Code::UnboundName, "no binding named 'x'", Span::new(0, 1));
        assert_eq!(d.render(&file), "t.bal:1:1: error[E_UNBOUND_NAME]: no binding named 'x'");
    }

    #[test]
    fn renders_a_note_line_per_secondary_span() {
        let file = SourceFile::new("t.bal".to_string(), "let m\nuse m\nuse m".to_string());
        let d = Diagnostic::new(Code::Reused, "already consumed", Span::new(12, 13))
            .with_secondary("first consumed here", Span::new(6, 7));
        let rendered = d.render(&file);
        assert_eq!(
            rendered,
            "t.bal:3:1: error[E_REUSED]: already consumed\nt.bal:2:1: note: first consumed here"
        );
    }
}
