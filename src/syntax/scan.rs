//! Generic token scanning.

use std::{cmp::Ordering, num::NonZero, str::Chars};

/// A scanner for reading tokens from UTF-8 text.
#[derive(Clone, Debug)]
pub struct Scanner<'s> {
    /// The full source text.
    src: &'s str,
    /// Iterator at the next char.
    chars: Chars<'s>,
    /// Start position of the current token.
    start: Pos,
    /// End position of the current token.
    end: Pos,
}

/// A source position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pos {
    /// Byte offset, starting at 0.
    offset: usize,
    /// Line number, starting at 1.
    line: NonZero<u32>,
    /// Column number, starting at 1.
    column: NonZero<u32>,
}

impl<'s> Scanner<'s> {
    /// Constructs a new scanner for the source text.
    pub fn new(src: &'s str) -> Self {
        let pos = Pos {
            offset: 0,
            line: NonZero::new(1).unwrap(),
            column: NonZero::new(1).unwrap(),
        };
        Scanner {
            src,
            chars: src.chars(),
            start: pos,
            end: pos,
        }
    }

    /// Returns the full source text.
    #[inline]
    pub fn src(&self) -> &'s str {
        self.src
    }

    /// Returns the text of the current token.
    #[inline]
    pub fn text(&self) -> &'s str {
        &self.src[self.start.offset..self.end.offset]
    }

    /// Returns the remaining text to be scanned.
    #[inline]
    pub fn rest(&self) -> &'s str {
        self.chars.as_str()
    }

    /// Returns the start position of the current token.
    #[inline]
    pub fn start(&self) -> Pos {
        self.start
    }

    /// Returns the end position of the current token.
    #[inline]
    pub fn end(&mut self) -> Pos {
        self.end
    }

    /// Returns the current offset into the source.
    #[inline]
    pub fn offset(&self) -> usize {
        self.end.offset
    }

    /// Starts scanning a new token.
    #[inline]
    pub fn start_next(&mut self) {
        self.start = self.end;
    }

    /// Backtracks to an earlier position in the current token.
    #[inline]
    pub fn backtrack(&mut self, end: Pos) {
        debug_assert!(self.start <= end, "backtracked before start");
        debug_assert!(end <= self.end, "backtracked after end");
        self.end = end;
    }

    /// Returns whether the scanner is at the end of the source.
    #[inline]
    pub fn eof(&self) -> bool {
        self.rest().is_empty()
    }

    /// Returns the next character without consuming it.
    #[inline]
    pub fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }

    /// Consumes and returns the next character.
    pub fn next(&mut self) -> Option<char> {
        let ch = self.chars.next()?;
        self.bump_pos(ch);
        Some(ch)
    }

    /// Consumes the next character.
    pub fn bump(&mut self) {
        debug_assert!(self.next().is_some());
    }

    /// Consumes the next character if it matches the predicate.
    pub fn bump_if<F: FnOnce(char) -> bool>(&mut self, predicate: F) -> bool {
        if let Some(ch) = self.peek()
            && predicate(ch)
        {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Consumes characters matching a predicate and returns the consumed text.
    pub fn bump_while<F: FnMut(char) -> bool>(&mut self, mut predicate: F) -> &'s str {
        let start = self.end.offset;
        while self.bump_if(&mut predicate) {}
        &self.src[start..self.end.offset]
    }

    /// Moves the end position by the width of the character.
    #[inline(always)]
    fn bump_pos(&mut self, ch: char) {
        self.end.column = self.end.column.saturating_add(1);
        if ch == '\n' {
            self.end.line = self.end.line.saturating_add(1);
            self.end.column = NonZero::new(1).unwrap();
        }
        self.end.offset = self.src.len() - self.chars.as_str().len();
    }
}

impl Pos {
    /// Returns the byte offset, starting at 0.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the line number, starting at 1.
    pub fn line(&self) -> usize {
        self.line.get() as usize
    }

    /// Returns the column number, starting at 1.
    pub fn column(&self) -> usize {
        self.column.get() as usize
    }
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
                panic!("compared positions from different sources: {self:?} and {other:?}");
            }
        }
        Some(ord)
    }
}
