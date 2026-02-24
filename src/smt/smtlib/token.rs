//! SMT-LIB tokens.
//!
//! Reference: [*The SMT-LIB Standard*, Version 2.7](https://smt-lib.org/papers/smt-lib-reference-v2.7-r2025-07-07.pdf)

use std::{
    error,
    fmt::{self, Write},
};

use crate::util::make_enum;

/// An SMT-LIB token. This does not cover fractional, hexadecimal, binary, and
/// reserved tokens or whitespace and comment ignored tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// Integer literal. It is always pretty-printed as decimal, not hexadecimal
    /// or binary.
    Numeral(i64),
    /// String literal.
    String(StringLit),
    /// Symbol.
    Symbol(Symbol),
    /// Keyword.
    Keyword(Keyword),
}

/// An SMT-LIB string literal.
#[derive(Clone, PartialEq, Eq)]
pub struct StringLit(String);

/// An SMT-LIB symbol.
#[derive(Clone, PartialEq, Eq)]
pub struct Symbol(String);

/// An SMT-LIB keyword.
#[derive(Clone, PartialEq, Eq)]
pub struct Keyword(String);

make_enum! {
    /// An SMT-LIB command name.
    pub enum CommandName;
    Assert => "assert",
    CheckSat => "check-sat",
    CheckSatAssuming => "check-sat-assuming",
    DeclareConst => "declare-const",
    DeclareDatatype => "declare-datatype",
    DeclareDatatypes => "declare-datatypes",
    DeclareFun => "declare-fun",
    DeclareSort => "declare-sort",
    DeclareSortParameter => "declare-sort-parameter",
    DefineConst => "define-const",
    DefineFun => "define-fun",
    DefineFunRec => "define-fun-rec",
    DefineFunsRec => "define-funs-rec",
    DefineSort => "define-sort",
    Echo => "echo",
    Exit => "exit",
    GetAssertions => "get-assertions",
    GetAssignment => "get-assignment",
    GetInfo => "get-info",
    GetModel => "get-model",
    GetOption => "get-option",
    GetProof => "get-proof",
    GetUnsatAssumptions => "get-unsat-assumptions",
    GetUnsatCore => "get-unsat-core",
    GetValue => "get-value",
    Pop => "pop",
    Push => "push",
    Reset => "reset",
    ResetAssertions => "reset-assertions",
    SetInfo => "set-info",
    SetLogic => "set-logic",
    SetOption => "set-option",
}

/// An error from constructing SMT-LIB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Non-printable char in SMT-LIB string literal.
    NonPrintableString,
    /// Unquotable char in SMT-LIB symbol.
    UnquotableSymbol,
    /// Invalid char in SMT-LIB keyword.
    InvalidKeyword,
}

impl StringLit {
    /// Constructs an SMT-LIB string literal, validated to the allowed grammar.
    pub fn new(s: String) -> Result<Self, Error> {
        if s.as_bytes()
            .iter()
            .all(|&b| is_printable(b) || is_whitespace(b))
        {
            Ok(StringLit(s))
        } else {
            Err(Error::NonPrintableString)
        }
    }
}

impl Symbol {
    /// Constructs an SMT-LIB symbol, validated to the allowed grammar.
    pub fn new(mut sym: String) -> Result<Self, Error> {
        if Self::is_simple_symbol(&sym) {
            Ok(Symbol(sym))
        } else if Self::can_quote(&sym) {
            sym.reserve(2);
            sym.insert(0, '|');
            sym.push('|');
            Ok(Symbol(sym))
        } else {
            Err(Error::UnquotableSymbol)
        }
    }

    fn is_simple_symbol(s: &str) -> bool {
        let s = s.as_bytes();
        !s.is_empty()
            && !s[0].is_ascii_digit()
            && s.iter()
                .all(|&b| b.is_ascii_alphanumeric() || is_special_char(b))
    }

    fn can_quote(s: &str) -> bool {
        s.as_bytes()
            .iter()
            .all(|&b| (is_printable(b) || is_whitespace(b)) && b != b'|' && b != b'\\')
    }
}

impl Keyword {
    /// Constructs an SMT-LIB keyword, validated to the allowed grammar.
    pub fn new(kw: String) -> Result<Self, Error> {
        if Symbol::is_simple_symbol(&kw) {
            Ok(Keyword(kw))
        } else {
            Err(Error::InvalidKeyword)
        }
    }
}

fn is_printable(b: u8) -> bool {
    !matches!(b, 0..=0x1F | 0x7F)
}
fn is_whitespace(b: u8) -> bool {
    matches!(b, b'\t' | b'\n' | b'\r' | b' ')
}
#[rustfmt::skip]
fn is_special_char(b: u8) -> bool {
    matches!(b,
        b'~' | b'!' | b'@' | b'$' | b'%' | b'^' | b'&' | b'*' | b'_'
            | b'-' | b'+' | b'=' | b'<' | b'>' | b'.' | b'?' | b'/')
}

impl fmt::Debug for StringLit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StringLit({self})")
    }
}

impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Symbol({self})")
    }
}

impl fmt::Debug for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Keyword({self})")
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Numeral(n) => write!(f, "{n}"),
            Token::String(s) => s.fmt(f),
            Token::Symbol(sym) => sym.fmt(f),
            Token::Keyword(kw) => kw.fmt(f),
        }
    }
}

impl fmt::Display for StringLit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('"')?;
        let mut s = self.0.as_str();
        while let Some((literal, rest)) = s.split_once('"') {
            f.write_str(literal)?;
            f.write_str("\"\"")?;
            s = rest;
        }
        f.write_str(s)?;
        f.write_char('"')
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ":{}", self.0)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::NonPrintableString => "non-printable char in SMT-LIB string literal",
            Error::UnquotableSymbol => "unquotable char in SMT-LIB symbol",
            Error::InvalidKeyword => "invalid char in SMT-LIB keyword",
        })
    }
}

impl error::Error for Error {}
