//! Lexical analysis of Mini-Alive source.

use std::{fmt, mem};

use crate::syntax::scan::{Scanner, Span};

/// A lexical unit of text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lexeme<'s> {
    /// Its token.
    pub tok: Token,
    /// Its source span.
    pub span: Span,
    /// Its source text.
    pub text: &'s str,
}

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
    LBracket,
    /// `]`
    RBracket,
    /// `,`
    Comma,
    /// `=`
    Eq,
    /// End of text
    Eof,
    /// Anything else is invalid
    Invalid,
}
const TOKEN_COUNT: usize = Token::Invalid as usize + 1;

/// A set of tokens.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TokenSet(u16);

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

    /// Lexes the next lexeme.
    pub fn next(&mut self) -> Lexeme<'s> {
        macro_rules! whitespace_pat(() => {
            ' ' | '\t' | '\n' | '\r' | '\0'
        });
        fn is_ident_rest(ch: char) -> bool {
            matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '$' | '.' |'_' | '-')
        }
        fn is_digit(ch: char) -> bool {
            matches!(ch, '0'..='9')
        }

        let tok = 'next: loop {
            self.scan.start_next();
            let Some(first) = self.scan.next() else {
                break Token::Eof;
            };
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
                '[' => Token::LBracket,
                ']' => Token::RBracket,
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
            break tok;
        };
        Lexeme {
            tok,
            span: self.scan.span(),
            text: self.scan.text(),
        }
    }
}

impl Token {
    /// Returns whether the token can have varying text.
    pub const fn can_vary(self) -> bool {
        token_set!(Ident | Int | GlobalName | LocalName | Label | Invalid).contains(self)
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Token::Ident => "identifier",
            Token::Int => "integer literal",
            Token::GlobalName => "global name (@)",
            Token::LocalName => "local name (%)",
            Token::Label => "label",
            Token::LParen => "`(`",
            Token::RParen => "`)`",
            Token::LBrace => "`{`",
            Token::RBrace => "`}`",
            Token::LBracket => "`[`",
            Token::RBracket => "`]`",
            Token::Comma => "`,`",
            Token::Eq => "`=`",
            Token::Eof => "EOF",
            Token::Invalid => "invalid token",
        })
    }
}

macro_rules! token_set(($($tok:ident)|*) => {
    const {
        crate::syntax::lex::TokenSet::empty()
            $(.insert(crate::syntax::lex::Token::$tok))*
    }
});
pub(crate) use token_set;

impl TokenSet {
    /// Constructs an empty token set.
    pub const fn empty() -> Self {
        TokenSet(0)
    }

    /// Constructs a token set containing one token.
    pub const fn one(tok: Token) -> Self {
        TokenSet::empty().insert(tok)
    }

    /// Returns whether the token is in the set.
    pub const fn contains(self, tok: Token) -> bool {
        self.0 & (1 << tok as u16) != 0
    }

    /// Inserts a token into the set.
    pub const fn insert(self, tok: Token) -> Self {
        Self(self.0 | (1 << tok as u16))
    }

    /// Combines two token sets.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Returns the number of tokens in the set.
    pub const fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Returns whether the set is empty.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl From<Token> for TokenSet {
    fn from(tok: Token) -> Self {
        TokenSet::one(tok)
    }
}

impl Iterator for TokenSet {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            return None;
        }
        let tok = self.0.trailing_zeros();
        self.0 &= self.0 - 1;
        Some(unsafe { mem::transmute(tok as u8) })
    }
}

impl fmt::Debug for TokenSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TokenSet(")?;
        let mut first = true;
        for i in 0..TOKEN_COUNT {
            let tok: Token = unsafe { mem::transmute(i as u8) };
            if self.contains(tok) {
                if !first {
                    f.write_str(" | ")?;
                }
                first = false;
                write!(f, "{tok:?}")?;
            }
        }
        f.write_str(")")
    }
}
