//! Source positions.

use std::{cmp::Ordering, fmt};

/// A source position range.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    /// Start position.
    pub start: Pos,
    /// End position.
    pub end: Pos,
}

/// A source position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pos {
    /// Byte offset, starting at 0.
    pub offset: usize,
    /// Line number, starting at 1.
    pub line: usize,
    /// Column number, starting at 1.
    pub column: usize,
}

impl PartialOrd for Pos {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let ord = self.offset.cmp(&other.offset);
        #[cfg(debug_assertions)]
        {
            let consistent = match ord {
                Ordering::Less => {
                    self.line < other.line || self.line == other.line && self.column < other.column
                }
                Ordering::Equal => self.line == other.line && self.column == other.column,
                Ordering::Greater => {
                    self.line > other.line || self.line == other.line && self.column > other.column
                }
            };
            if !consistent {
                panic!("compared positions from different sources: {self} and {other}");
            }
        }
        Some(ord)
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.end.offset == self.start.offset + 1
            && self.start.line == self.end.line
            && self.end.column == self.start.column + 1
        {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}
