//! Generic token scanning.

use std::str::Chars;

use crate::syntax::source::{Pos, SourceFile, Span};

/// A scanner for reading tokens from UTF-8 text.
#[derive(Clone, Debug)]
pub struct Scanner<'s> {
    /// The source file being scanned.
    src: &'s SourceFile,
    /// Iterator at the next char.
    chars: Chars<'s>,
    /// Start position of the current token.
    start: Pos,
    /// End position of the current token.
    end: Pos,
}

impl<'s> Scanner<'s> {
    /// Constructs a new scanner for the source text.
    pub fn new(src: &'s SourceFile) -> Self {
        let pos = Pos { offset: 0 };
        Scanner {
            src,
            chars: src.text().chars(),
            start: pos,
            end: pos,
        }
    }

    /// Gets the source file.
    #[inline]
    pub fn src(&self) -> &'s SourceFile {
        self.src
    }

    /// Gets the text of the current token.
    #[inline]
    pub fn text(&self) -> &'s str {
        &self.src.text()[self.start.offset..self.end.offset]
    }

    /// Gets the source position range of the current token.
    #[inline]
    pub fn span(&self) -> Span {
        Span {
            start: self.start,
            end: self.end,
        }
    }

    /// Starts scanning a new token.
    #[inline]
    pub fn start_next(&mut self) {
        self.start = self.end;
    }

    /// Gets the next character without consuming it.
    #[inline]
    pub fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }

    /// Consumes and returns the next character.
    pub fn next(&mut self) -> Option<char> {
        let ch = self.chars.next()?;
        self.end.offset = self.src.text().len() - self.chars.as_str().len();
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
    pub fn bump_while<F: FnMut(char) -> bool>(&mut self, mut predicate: F) -> bool {
        let mut moved = false;
        while self.bump_if(&mut predicate) {
            moved = true;
        }
        moved
    }
}
