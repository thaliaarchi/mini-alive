//! Lexical analysis.

use crate::syntax::scan::{Scanner, Span};

/// A lexical token.
///
/// Skipped:
/// - Whitespace: `[ \t\n\r\0]+`
/// - Comment: `";" [^\n\r]* | "/*" .*? "*/"`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    /// Identifier: `[a-zA-Z$._-][a-zA-Z0-9$._-]*`
    Ident,
    /// Integer literal: `-?[0-9]+`
    Int,
    /// Global name: `"@" ([a-zA-Z$._-][a-zA-Z0-9$._-]* | [0-9]+)`
    GlobalName,
    /// Local name: `"%" ([a-zA-Z$._-][a-zA-Z0-9$._-]* | [0-9]+)`
    LocalName,
    /// Label: `([a-zA-Z$._-][a-zA-Z0-9$._-]* | [0-9]+) ":"`
    Label,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBrack,
    /// `]`
    RBrack,
    /// `,`
    Comma,
    /// `=`
    Eq,
    /// Anything else is invalid
    Invalid,
}

/// A lexer for Mini-Alive tokens.
pub struct Lexer<'s> {
    scan: Scanner<'s>,
}

impl<'s> Lexer<'s> {
    /// Constructs a lexer for Mini-Alive source.
    pub fn new(src: &'s str) -> Lexer<'s> {
        Lexer {
            scan: Scanner::new(src),
        }
    }

    /// Gets the text of the current token.
    pub fn text(&self) -> &'s str {
        self.scan.text()
    }

    /// Gets the source position range of the current token.
    pub fn span(&self) -> Span {
        self.scan.span()
    }
}

impl<'s> Iterator for Lexer<'s> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        macro_rules! whitespace_pat(() => {
            ' ' | '\t' | '\n' | '\r' | '\0'
        });
        fn is_ident_rest(ch: char) -> bool {
            matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '$' | '.' |'_' | '-')
        }
        fn is_digit(ch: char) -> bool {
            matches!(ch, '0'..='9')
        }

        'next: loop {
            self.scan.start_next();
            let first = self.scan.next()?;
            let tok = match first {
                whitespace_pat!() => {
                    self.scan.bump_while(|ch| matches!(ch, whitespace_pat!()));
                    continue;
                }
                'a'..='z' | 'A'..='Z' | '$' | '.' | '_' => {
                    self.scan.bump_while(is_ident_rest);
                    if self.scan.bump_if(|ch| ch == ':') {
                        Token::Label
                    } else {
                        Token::Ident
                    }
                }
                '-' | '0'..='9' => {
                    let tok = if self.scan.bump_while(is_digit) || first != '-' {
                        Token::Int
                    } else {
                        self.scan.bump_while(is_ident_rest);
                        Token::Ident
                    };
                    if self.scan.bump_if(|ch| ch == ':') {
                        Token::Label
                    } else {
                        tok
                    }
                }
                '@' | '%' => {
                    if !self.scan.bump_while(is_digit) {
                        self.scan.bump_while(is_ident_rest);
                    }
                    if first == '@' {
                        Token::GlobalName
                    } else {
                        Token::LocalName
                    }
                }
                '(' => Token::LParen,
                ')' => Token::RParen,
                '{' => Token::LBrace,
                '}' => Token::RBrace,
                '[' => Token::LBrack,
                ']' => Token::RBrack,
                ',' => Token::Comma,
                '=' => Token::Eq,
                ';' => {
                    self.scan.bump_while(|ch| ch != '\n' && ch != '\r');
                    continue;
                }
                '/' => {
                    if self.scan.bump_if(|ch| ch == '*') {
                        while let Some(ch) = self.scan.next() {
                            if ch == '*' && self.scan.bump_if(|ch| ch == '/') {
                                continue 'next;
                            }
                        }
                    }
                    Token::Invalid
                }
                '"' => {
                    self.scan.bump_while(|ch| ch != '"');
                    self.scan.bump_if(|ch| ch == '"');
                    Token::Invalid
                }
                _ => Token::Invalid,
            };
            return Some(tok);
        }
    }
}
