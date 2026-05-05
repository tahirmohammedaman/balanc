//! Byte-offset source spans. Only `diag` turns a `Span` into line/column; every other
//! stage treats it as an opaque range to carry along.

/// A half-open byte range `[lo, hi)` into a single `SourceFile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub lo: u32,
    pub hi: u32,
}

impl Span {
    pub fn new(lo: u32, hi: u32) -> Self {
        debug_assert!(lo <= hi);
        Span { lo, hi }
    }

    /// The smallest span covering both `self` and `other`.
    pub fn to(self, other: Span) -> Span {
        Span::new(self.lo.min(other.lo), self.hi.max(other.hi))
    }
}

/// A single loaded source file, kept alive for the lifetime of a compilation so spans
/// can be resolved back to text.
pub struct SourceFile {
    pub name: String,
    pub text: String,
}

impl SourceFile {
    pub fn new(name: String, text: String) -> Self {
        SourceFile { name, text }
    }

    pub fn slice(&self, span: Span) -> &str {
        &self.text[span.lo as usize..span.hi as usize]
    }

    /// 1-based (line, column) of a byte offset, for diagnostic rendering.
    pub fn line_col(&self, offset: u32) -> (u32, u32) {
        let offset = offset as usize;
        let mut line = 1u32;
        let mut col = 1u32;
        for ch in self.text[..offset.min(self.text.len())].chars() {
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }
}
