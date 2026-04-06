//! Source positions.

use std::ffi::{OsStr, OsString};

/// The text and metadata for a single source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    /// The source filename.
    filename: OsString,
    /// The full source text.
    text: String,
    /// The byte offsets of the start of each line.
    line_offsets: Vec<usize>,
}

/// A source position range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    /// Start position, inclusive.
    pub start: Pos,
    /// End position, exclusive.
    pub end: Pos,
}

/// A source position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub struct Pos {
    /// Byte offset, starting at 0.
    pub offset: usize,
}

/// The line/column coordinates of a source position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LineCol {
    /// Line number, starting at 1.
    pub line: usize,
    /// Column number, starting at 1.
    pub column: usize,
}

impl SourceFile {
    /// Constructs a source file from its text.
    pub fn new(text: String, filename: OsString) -> Self {
        let mut line_offsets = Vec::new();
        line_offsets.push(0);
        for (offset, ch) in text.char_indices() {
            if ch == '\n' {
                line_offsets.push(offset + 1);
            }
        }
        SourceFile {
            filename,
            text,
            line_offsets,
        }
    }

    /// Gets the source filename.
    pub fn filename(&self) -> &OsStr {
        &self.filename
    }

    /// Gets the full source text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Gets the text of a source line, excluding the trailing line ending.
    pub fn line_text(&self, line: usize) -> &str {
        let start = self.line_offsets[line - 1];
        let end = self
            .line_offsets
            .get(line)
            .copied()
            .unwrap_or(self.text.len());
        let mut text = &self.text[start..end];
        if let Some(rest) = text.strip_suffix('\n') {
            text = rest;
        }
        if let Some(rest) = text.strip_suffix('\r') {
            text = rest;
        }
        text
    }

    /// Resolves a byte offset to line/column coordinates.
    pub fn position(&self, offset: usize) -> LineCol {
        let line = self.line_index(offset);
        let line_start = self.line_offsets[line];
        let column = self.text[line_start..offset].chars().count();
        LineCol {
            line: line + 1,
            column: column + 1,
        }
    }

    fn line_index(&self, offset: usize) -> usize {
        self.line_offsets.partition_point(|&start| start <= offset) - 1
    }
}

impl Span {
    /// Resolves the start position to line/column coordinates.
    pub fn start_position(&self, src: &SourceFile) -> LineCol {
        src.position(self.start.offset)
    }

    /// Resolves the end position to line/column coordinates.
    pub fn end_position(&self, src: &SourceFile) -> LineCol {
        if self.end.offset != 0 && src.text().as_bytes()[self.end.offset - 1] == b'\n' {
            let line = src.line_index(self.end.offset) - 1;
            let line_start = src.line_offsets[line];
            let column = src.text[line_start..self.end.offset].chars().count();
            LineCol {
                line: line + 1,
                column: column + 1,
            }
        } else {
            src.position(self.end.offset)
        }
    }

    /// Gets the source text for the span.
    pub fn text<'s>(&self, src: &'s SourceFile) -> &'s str {
        &src.text[self.start.offset..self.end.offset]
    }
}
