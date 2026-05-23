//! User-facing diagnostics. Every `Diagnostic` carries a stable `Code` and a real
//! `Span` (invariant 6) — never a synthetic or zero span.
//!
//! Rendering (Slice 6) is rustc-lite: a `file:line:col` header, the offending source
//! line with a caret under the exact span, one `note:` block per secondary span (its
//! own location/line/caret, for e.g. both use-sites of an `E_REUSED` value), and a
//! trailing `= note:` line with fixed, per-`Code` guidance (`Code::note`) — general
//! advice for *this class* of error, not a restatement of the specific message above
//! it. `render_all_json` renders the same information as one JSON object per
//! diagnostic instead, for editor consumption (`--json`, see `main.rs`).

pub mod codes;

use crate::span::{SourceFile, Span};
pub use codes::Code;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: Code,
    pub message: String,
    pub span: Span,
    /// Extra `(label, span)` pairs — e.g. `("first consumed here", span)` — each
    /// rendered as its own located, captioned source block after the primary one.
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

    /// Plain-text rendering: header, primary source block, a static per-code note,
    /// then one `note: <label>` block per secondary span. Multiple diagnostics are
    /// joined by `render_all` with a blank line between them.
    pub fn render(&self, file: &SourceFile) -> String {
        let gutter = self.gutter_width(file);
        let mut lines = vec![format!("error[{}]: {}", self.code, self.message)];
        lines.extend(span_block_lines(file, self.span, gutter));
        lines.push(format!("{:gutter$} = note: {}", "", self.code.note(), gutter = gutter));
        for (label, span) in &self.secondary {
            lines.push(String::new());
            lines.push(format!("note: {label}"));
            lines.extend(span_block_lines(file, *span, gutter));
        }
        lines.join("\n")
    }

    /// One JSON object: `code`, `message`, `file`, 1-based `line`/`col`, the raw byte
    /// `span`, and a `secondary` array shaped the same way (minus `message`, plus
    /// `label`). No `note` field — `Code::note` is a fixed lookup an editor can do
    /// itself from `code`, cheaper than repeating the same string per occurrence.
    pub fn to_json(&self, file: &SourceFile) -> String {
        let (line, col) = file.line_col(self.span.lo);
        let mut out = format!(
            "{{\"code\":\"{}\",\"message\":{},\"file\":{},\"line\":{},\"col\":{},\"span\":{{\"lo\":{},\"hi\":{}}},\"secondary\":[",
            self.code, json_escape(&self.message), json_escape(&file.name), line, col, self.span.lo, self.span.hi,
        );
        for (i, (label, span)) in self.secondary.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let (line, col) = file.line_col(span.lo);
            out.push_str(&format!(
                "{{\"label\":{},\"line\":{},\"col\":{},\"span\":{{\"lo\":{},\"hi\":{}}}}}",
                json_escape(label),
                line,
                col,
                span.lo,
                span.hi,
            ));
        }
        out.push_str("]}");
        out
    }

    /// The gutter (line-number column) is sized to the widest line number this
    /// diagnostic touches, primary or secondary, so every block it prints lines up.
    fn gutter_width(&self, file: &SourceFile) -> usize {
        let mut max_line = file.line_col(self.span.lo).0;
        for (_, span) in &self.secondary {
            max_line = max_line.max(file.line_col(span.lo).0);
        }
        max_line.to_string().len()
    }
}

/// The `--> file:line:col` / ` |` / `N | source` / ` | ^^^^` block shared by a
/// diagnostic's primary span and each of its secondary spans.
fn span_block_lines(file: &SourceFile, span: Span, gutter: usize) -> Vec<String> {
    let (line, col) = file.line_col(span.lo);
    let (line_start, line_end) = line_bounds(file, line);
    let source_line = &file.text[line_start..line_end];
    let caret = caret_line(source_line, line_start, span);
    vec![
        format!("{:gutter$} --> {}:{}:{}", "", file.name, line, col, gutter = gutter),
        format!("{:gutter$} |", "", gutter = gutter),
        format!("{:>gutter$} | {}", line, source_line, gutter = gutter),
        format!("{:gutter$} | {}", "", caret, gutter = gutter),
    ]
}

/// Byte range `[start, end)` of 1-based `line`, excluding its trailing `\n`.
fn line_bounds(file: &SourceFile, line: u32) -> (usize, usize) {
    let bytes = file.text.as_bytes();
    let mut start = 0usize;
    let mut current = 1u32;
    if line > 1 {
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\n' {
                current += 1;
                if current == line {
                    start = i + 1;
                    break;
                }
            }
        }
    }
    let end = file.text[start..].find('\n').map(|i| start + i).unwrap_or(file.text.len());
    (start, end)
}

/// Spaces (and tabs, to keep column alignment on a real terminal) up to `span`'s start
/// column, then one `^` per character `span` covers on this line — at least one, even
/// for a zero-width span (e.g. an end-of-file diagnostic), so there is always
/// something to point at.
fn caret_line(source_line: &str, line_start: usize, span: Span) -> String {
    let mut prefix = String::new();
    let mut carets = String::new();
    for (i, ch) in source_line.char_indices() {
        let byte_off = (line_start + i) as u32;
        if byte_off < span.lo {
            prefix.push(if ch == '\t' { '\t' } else { ' ' });
        } else if byte_off < span.hi {
            carets.push('^');
        }
    }
    if carets.is_empty() {
        carets.push('^');
    }
    format!("{prefix}{carets}")
}

/// Escapes `s` as a JSON string literal, quotes included.
pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Sort diagnostics by span so output order is deterministic regardless of which
/// stage or traversal order produced them.
pub fn sort_by_span(diags: &mut [Diagnostic]) {
    diags.sort_by_key(|d| (d.span.lo, d.span.hi));
}

/// Renders every diagnostic (assumed already sorted, per `sort_by_span`) as plain
/// text, one blank line apart, with a single trailing newline.
pub fn render_all(diags: &[Diagnostic], file: &SourceFile) -> String {
    let mut out = diags.iter().map(|d| d.render(file)).collect::<Vec<_>>().join("\n\n");
    out.push('\n');
    out
}

/// Renders every diagnostic (assumed already sorted) as a `{"ok":false,"diagnostics":
/// [...]}` JSON envelope, with a single trailing newline.
pub fn render_all_json(diags: &[Diagnostic], file: &SourceFile) -> String {
    let body = diags.iter().map(|d| d.to_json(file)).collect::<Vec<_>>().join(",");
    format!("{{\"ok\":false,\"diagnostics\":[{body}]}}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::SourceFile;

    #[test]
    fn renders_a_caret_under_the_primary_span_and_a_note() {
        let file = SourceFile::new("t.bal".to_string(), "let x = y\n".to_string());
        let d = Diagnostic::new(Code::UnboundName, "no binding named 'y'", Span::new(8, 9));
        let rendered = d.render(&file);
        let lines: Vec<&str> = rendered.lines().collect();
        assert_eq!(lines[0], "error[E_UNBOUND_NAME]: no binding named 'y'");
        assert_eq!(lines[1], "  --> t.bal:1:9");
        assert_eq!(lines[2], "  |");
        assert_eq!(lines[3], "1 | let x = y");
        // The caret sits directly under the 'y' the source line printed above it.
        let source_col = lines[3].find('y').unwrap();
        let caret_col = lines[4].find('^').unwrap();
        assert_eq!(source_col, caret_col);
        assert_eq!(lines[4].matches('^').count(), 1);
        assert_eq!(
            lines[5],
            "  = note: check the spelling, or add a 'let' binding for this name earlier in the transaction"
        );
    }

    #[test]
    fn renders_a_located_block_per_secondary_span() {
        let file = SourceFile::new("t.bal".to_string(), "let m\nuse m\nuse m".to_string());
        let d = Diagnostic::new(Code::Reused, "already consumed", Span::new(12, 13))
            .with_secondary("first consumed here", Span::new(6, 7));
        let rendered = d.render(&file);
        assert!(rendered.starts_with("error[E_REUSED]: already consumed\n"));
        assert!(rendered.contains(" --> t.bal:3:1\n"));
        assert!(rendered.contains("3 | use m\n"));
        assert!(rendered.contains("\nnote: first consumed here\n"));
        assert!(rendered.contains(" --> t.bal:2:1\n"));
        assert!(rendered.contains("2 | use m\n"));
    }

    #[test]
    fn gutter_widens_to_fit_the_largest_line_number_touched() {
        let mut text = String::new();
        for _ in 0..10 {
            text.push_str("let a = b;\n");
        }
        text.push_str("use x\n"); // line 11
        let file = SourceFile::new("t.bal".to_string(), text);
        let span = Span::new((file.text.len() - "x\n".len()) as u32, (file.text.len() - "\n".len()) as u32);
        let d = Diagnostic::new(Code::UnboundName, "no binding named 'x'", span);
        let rendered = d.render(&file);
        // A two-digit line number ("11") forces a two-column gutter throughout.
        assert!(rendered.contains("11 | use x"));
        assert!(rendered.contains("   --> t.bal:11:5"));
    }

    #[test]
    fn a_zero_width_end_of_file_span_still_gets_a_caret() {
        let file = SourceFile::new("t.bal".to_string(), "txn \"t\" {".to_string());
        let eof = Span::new(9, 9);
        let d = Diagnostic::new(Code::ParseUnexpectedEof, "expected '}'", eof);
        let rendered = d.render(&file);
        let lines: Vec<&str> = rendered.lines().collect();
        assert_eq!(lines[3], "1 | txn \"t\" {");
        // The source line has nothing at byte 9 (one past its last char), so the
        // caret falls one column past the line's printed end rather than under it.
        assert_eq!(lines[4].matches('^').count(), 1);
        assert_eq!(lines[4].trim_end().len(), lines[3].len() + 1);
    }

    #[test]
    fn to_json_carries_code_location_span_and_secondary_labels() {
        let file = SourceFile::new("t.bal".to_string(), "let m\nuse m\nuse m".to_string());
        let d = Diagnostic::new(Code::Reused, "already consumed", Span::new(12, 13))
            .with_secondary("first consumed here", Span::new(6, 7));
        let json = d.to_json(&file);
        assert_eq!(
            json,
            "{\"code\":\"E_REUSED\",\"message\":\"already consumed\",\"file\":\"t.bal\",\"line\":3,\"col\":1,\
             \"span\":{\"lo\":12,\"hi\":13},\"secondary\":[{\"label\":\"first consumed here\",\"line\":2,\"col\":1,\
             \"span\":{\"lo\":6,\"hi\":7}}]}"
        );
    }

    #[test]
    fn json_escape_handles_quotes_backslashes_and_control_characters() {
        assert_eq!(json_escape("a \"quoted\" \\ thing\n"), "\"a \\\"quoted\\\" \\\\ thing\\n\"");
    }

    #[test]
    fn render_all_separates_diagnostics_with_a_blank_line_and_ends_with_one_newline() {
        let file = SourceFile::new("t.bal".to_string(), "a\nb\n".to_string());
        let diags = vec![
            Diagnostic::new(Code::UnboundName, "first", Span::new(0, 1)),
            Diagnostic::new(Code::UnboundName, "second", Span::new(2, 3)),
        ];
        let rendered = render_all(&diags, &file);
        assert!(rendered.contains("first\n"));
        assert!(rendered.contains("\n\nerror[E_UNBOUND_NAME]: second"));
        assert!(rendered.ends_with('\n') && !rendered.ends_with("\n\n"));
    }

    #[test]
    fn render_all_json_wraps_diagnostics_in_an_envelope() {
        let file = SourceFile::new("t.bal".to_string(), "a\n".to_string());
        let diags = vec![Diagnostic::new(Code::UnboundName, "no binding named 'a'", Span::new(0, 1))];
        let rendered = render_all_json(&diags, &file);
        assert_eq!(
            rendered,
            "{\"ok\":false,\"diagnostics\":[{\"code\":\"E_UNBOUND_NAME\",\"message\":\"no binding named 'a'\",\
             \"file\":\"t.bal\",\"line\":1,\"col\":1,\"span\":{\"lo\":0,\"hi\":1},\"secondary\":[]}]}\n"
        );
    }
}
