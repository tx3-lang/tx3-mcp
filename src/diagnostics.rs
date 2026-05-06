use std::ops::Range;

use miette::Diagnostic as MietteDiagnostic;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Advice,
}

impl From<miette::Severity> for Severity {
    fn from(s: miette::Severity) -> Self {
        match s {
            miette::Severity::Error => Severity::Error,
            miette::Severity::Warning => Severity::Warning,
            miette::Severity::Advice => Severity::Advice,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagSpan {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    pub spans: Vec<DiagSpan>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<Diagnostic>,
}

/// Precomputed line-start byte offsets for fast byte→(line,col) conversion.
/// Uses chars() so multibyte source is handled correctly.
pub struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    /// Convert a byte offset to (line, column), both 1-based.
    /// Column counts characters (not bytes) within the line so multibyte chars
    /// don't inflate the column number.
    pub fn position(&self, source: &str, byte: usize) -> (usize, usize) {
        let line_idx = match self.line_starts.binary_search(&byte) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line_start = self.line_starts[line_idx];
        let line_bytes = &source.as_bytes()[line_start..byte.min(source.len())];
        let col = std::str::from_utf8(line_bytes)
            .map(|s| s.chars().count())
            .unwrap_or(byte - line_start);
        (line_idx + 1, col + 1)
    }
}

pub fn from_miette<E: MietteDiagnostic + ?Sized>(
    err: &E,
    source: Option<&str>,
    path: Option<&str>,
) -> Diagnostic {
    let severity = err.severity().unwrap_or(miette::Severity::Error).into();
    let code = err.code().map(|c| c.to_string());
    let message = err.to_string();
    let help = err.help().map(|h| h.to_string());
    let url = err.url().map(|u| u.to_string());

    let spans = match (err.labels(), source) {
        (Some(labels), Some(src)) => {
            let index = LineIndex::new(src);
            labels
                .map(|l| {
                    span_from_labeled(
                        &index,
                        src,
                        l.inner().offset()..l.inner().offset() + l.inner().len(),
                        l.label().map(|s| s.to_string()),
                    )
                })
                .collect()
        }
        _ => Vec::new(),
    };

    let related = err
        .related()
        .map(|iter| {
            iter.map(|child| from_miette(child, source, path))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Diagnostic {
        severity,
        code,
        message,
        help,
        url,
        source_path: path.map(str::to_string),
        spans,
        related,
    }
}

fn span_from_labeled(
    index: &LineIndex,
    source: &str,
    range: Range<usize>,
    label: Option<String>,
) -> DiagSpan {
    let (start_line, start_col) = index.position(source, range.start);
    let (end_line, end_col) = index.position(source, range.end);
    DiagSpan {
        start_line,
        start_col,
        end_line,
        end_col,
        start_byte: range.start,
        end_byte: range.end,
        label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_index_ascii() {
        let src = "abc\ndef\nghi";
        let idx = LineIndex::new(src);
        assert_eq!(idx.position(src, 0), (1, 1));
        assert_eq!(idx.position(src, 2), (1, 3));
        assert_eq!(idx.position(src, 4), (2, 1));
        assert_eq!(idx.position(src, 8), (3, 1));
    }

    #[test]
    fn line_index_multibyte() {
        // "héllo" — the é is 2 bytes (0xc3 0xa9). 'l' starts at byte 3.
        let src = "héllo\nworld";
        let idx = LineIndex::new(src);
        // byte 3 is 'l' (the first 'l' after é); char column should be 3.
        assert_eq!(idx.position(src, 3), (1, 3));
        // byte 6 is the newline-after-o handling: "héllo" is 6 bytes, then '\n' at 6.
        // First byte of "world" is 7.
        assert_eq!(idx.position(src, 7), (2, 1));
    }
}
