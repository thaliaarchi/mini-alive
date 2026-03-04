//! SMT-LIB S-expression AST.
//!
//! Reference: [*The SMT-LIB Standard*, Version 2.7](https://smt-lib.org/papers/smt-lib-reference-v2.7-r2025-07-07.pdf)

use std::{
    error,
    fmt::{self, Write},
};

use crate::{smt::smtlib::ListStyle, util::make_enum};

// TODO:
// - Spec ambiguities:
//   - Can keywords start with a digit? Simple symbols cannot start with a digit
//     and keywords are `:<simple_symbol>`, but the `:56` example keyword
//     suggests keywords should allow leading digits.
//   - Can keywords be reserved words? Simple symbols cannot be reserved words,
//     but they should be fine as keywords.
//   - Symbols starting with `@` or `.` are reserved for solver use, but section
//     3.1 writes that this applies only to simple symbols, which would make the
//     interpretation of such symbols inconsistent, depending on whether they're
//     quoted. Appendix B writes that this applies to both simple and quoted
//     symbols.

/// An S-expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SExp {
    /// Atomic value.
    Atom(Atom),
    /// Parenthesized list.
    List(List),
}

/// An S-expression list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List {
    /// The elements of the list.
    pub elems: Vec<SExp>,
    /// The style for pretty-printing the list.
    pub style: ListStyle,
}

/// An SMT-LIB atomic value. This does not cover fractional, hexadecimal,
/// binary, and reserved tokens or whitespace and comment ignored tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Atom {
    /// Integer literal. It is always pretty-printed as decimal, not hexadecimal
    /// or binary.
    Numeral(i64),
    /// String literal.
    String(StringLit),
    /// Symbol.
    Symbol(Symbol),
    /// Keyword.
    Keyword(Keyword),
    /// Reserved word.
    Reserved(Reserved),
    /// Command name.
    CommandName(CommandName),
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
    /// An SMT-LIB reserved word.
    pub enum Reserved;
    Binary => "BINARY",
    Decimal => "DECIMAL",
    Hexadecimal => "HEXADECIMAL",
    Numeral => "NUMERAL",
    String => "STRING",
    Underscore => "_",
    Bang => "!",
    As => "as",
    Lambda => "lambda",
    Let => "let",
    Exists => "exists",
    Forall => "forall",
    Match => "match",
    Par => "par",
}

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
    /// Non-printable char in SMT-LIB symbol.
    NonPrintableSymbol,
    /// Unquotable char in SMT-LIB symbol.
    UnquotableSymbol,
    /// SMT-LIB symbol is reserved for solver use.
    SolverReservedSymbol,
    /// SMT-LIB keyword is empty.
    EmptyKeyword,
    /// Invalid char in SMT-LIB keyword.
    InvalidKeyword,
}

impl<T: Into<Atom>> From<T> for SExp {
    fn from(atom: T) -> Self {
        SExp::Atom(atom.into())
    }
}

impl From<List> for SExp {
    fn from(list: List) -> Self {
        SExp::List(list)
    }
}

macro_rules! impl_from_for_atom(($($Variant:ident($Ty:ty)),* $(,)?) => {
    $(impl From<$Ty> for Atom {
        fn from(value: $Ty) -> Self {
            Atom::$Variant(value)
        }
    })*
});
impl_from_for_atom! {
    Numeral(i64),
    String(StringLit),
    Symbol(Symbol),
    Keyword(Keyword),
    Reserved(Reserved),
    CommandName(CommandName),
}

macro_rules! printable_pat(() => {
    // All non-ASCII Unicode is considered printable, so can be processed by
    // UTF-8 bytes.
    0x21..=0x7E | 0x80..
});
macro_rules! whitespace_pat(() => {
    b'\t' | b'\n' | b'\r' | b' '
});
macro_rules! alphanumeric_pat(() => {
    b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9'
});
macro_rules! special_char_pat(() => {
    b'~' | b'!' | b'@' | b'$' | b'%' | b'^' | b'&' | b'*' | b'_'
        | b'-' | b'+' | b'=' | b'<' | b'>' | b'.' | b'?' | b'/'
});

impl StringLit {
    /// Constructs an SMT-LIB string literal or panics if invalid.
    pub fn new<T: Into<String>>(s: T) -> Self {
        match StringLit::try_new(s) {
            Ok(s) => s,
            Err(err) => panic!("{err}"),
        }
    }

    /// Constructs an SMT-LIB string literal, validated to the allowed grammar.
    pub fn try_new<T: Into<String>>(s: T) -> Result<Self, Error> {
        StringLit::try_new_(s.into())
    }
    fn try_new_(s: String) -> Result<Self, Error> {
        if s.as_bytes()
            .iter()
            .all(|&b| matches!(b, printable_pat!() | whitespace_pat!()))
        {
            Ok(StringLit(s))
        } else {
            Err(Error::NonPrintableString)
        }
    }
}

impl Symbol {
    /// Constructs an SMT-LIB symbol or panics if invalid.
    pub fn new<T: Into<String>>(sym: T) -> Self {
        match Symbol::try_new(sym) {
            Ok(sym) => sym,
            Err(err) => panic!("{err}"),
        }
    }

    /// Constructs an SMT-LIB symbol, validated to the allowed grammar.
    pub fn try_new<T: Into<String>>(sym: T) -> Result<Self, Error> {
        Symbol::try_new_(sym.into())
    }
    fn try_new_(mut sym: String) -> Result<Self, Error> {
        let bytes = sym.as_bytes();
        let mut quoted = false;
        if bytes.is_empty() {
            quoted = true;
        } else {
            if bytes[0].is_ascii_digit() {
                quoted = true;
            }
            for &b in bytes {
                match b {
                    alphanumeric_pat!() | special_char_pat!() => {}
                    b'|' | b'\\' => return Err(Error::UnquotableSymbol),
                    printable_pat!() | whitespace_pat!() => quoted = true,
                    _ => return Err(Error::NonPrintableSymbol),
                }
            }
            if matches!(bytes[0], b'@' | b'.') {
                return Err(Error::SolverReservedSymbol);
            }
            if Reserved::from_str(&sym).is_some() {
                quoted = true;
            }
        }
        if quoted {
            sym.reserve(2);
            sym.insert(0, '|');
            sym.push('|');
        }
        Ok(Symbol(sym))
    }
}

impl Keyword {
    /// Constructs an SMT-LIB keyword or panics if invalid.
    pub fn new<T: Into<String>>(kw: T) -> Self {
        match Keyword::try_new(kw) {
            Ok(kw) => kw,
            Err(err) => panic!("{err}"),
        }
    }

    /// Constructs an SMT-LIB keyword, validated to the allowed grammar.
    pub fn try_new<T: Into<String>>(kw: T) -> Result<Self, Error> {
        Keyword::try_new_(kw.into())
    }
    fn try_new_(kw: String) -> Result<Self, Error> {
        if kw.is_empty() {
            Err(Error::EmptyKeyword)
        } else if kw
            .as_bytes()
            .iter()
            .all(|&b| matches!(b, alphanumeric_pat!() | special_char_pat!()))
        {
            Ok(Keyword(kw))
        } else {
            Err(Error::InvalidKeyword)
        }
    }
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

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Numeral(n) => write!(f, "{n}"),
            Atom::String(s) => s.fmt(f),
            Atom::Symbol(sym) => sym.fmt(f),
            Atom::Keyword(kw) => kw.fmt(f),
            Atom::Reserved(reserved) => reserved.fmt(f),
            Atom::CommandName(name) => name.fmt(f),
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
            Error::NonPrintableSymbol => "non-printable char in SMT-LIB symbol",
            Error::UnquotableSymbol => "unquotable char in SMT-LIB symbol",
            Error::SolverReservedSymbol => "SMT-LIB symbol is reserved for solver use",
            Error::EmptyKeyword => "SMT-LIB keyword is empty",
            Error::InvalidKeyword => "invalid char in SMT-LIB keyword",
        })
    }
}

impl error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    // Examples from The SMT-LIB Standard, Version 2.7, 3.1 Lexicon:

    #[test]
    fn spec_example_strings() {
        let tests = [
            ("this is a string literal", r#""this is a string literal""#),
            ("", r#""""#),
            (
                r#"She said: "Bye bye" and left."#,
                r#""She said: ""Bye bye"" and left.""#,
            ),
            (
                "this is a string literal\nwith a line break in it",
                "\"this is a string literal\nwith a line break in it\"",
            ),
            (
                r#"\n, \012, \x0A, and \u0008 are not escape sequences"#,
                r#""\n, \012, \x0A, and \u0008 are not escape sequences""#,
            ),
        ];
        for (raw, literal) in tests {
            assert_eq!(
                StringLit::try_new(raw).map(|s| s.to_string()).as_deref(),
                Ok(literal),
                "StringLit::try_new({raw:?})",
            );
        }
    }

    #[test]
    fn spec_example_symbols() {
        let tests = [
            // Simple symbols:
            ("+", "+"),
            ("<=", "<="),
            ("x", "x"),
            ("plus", "plus"),
            ("**", "**"),
            ("$", "$"),
            ("<", "<"),
            ("sas", "sas"),
            ("<adf>", "<adf>"),
            ("abc77", "abc77"),
            ("*$s&6", "*$s&6"),
            // (".aaa", ".aaa"), // Solver-reserved
            // (".8", ".8"),     // Solver-reserved
            ("+34", "+34"),
            ("-32", "-32"),
            // Quoted symbols:
            ("this is a quoted symbol", "|this is a quoted symbol|"),
            ("so is\n this one", "|so is\n this one|"),
            ("", "||"),
            (" \" can occur too", "| \" can occur too|"),
            (
                "af klj^*0asfe2(&*)&(#^$>>>?\"']]984",
                "|af klj^*0asfe2(&*)&(#^$>>>?\"']]984|",
            ),
        ];
        for (raw, literal) in tests {
            assert_eq!(
                Symbol::try_new(raw).map(|sym| sym.to_string()).as_deref(),
                Ok(literal),
                "Symbol::try_new({raw:?})",
            );
        }
    }

    #[test]
    fn spec_example_keywords() {
        let tests = [
            ("date", ":date"),
            ("a2", ":a2"),
            ("foo-bar", ":foo-bar"),
            ("<=", ":<="),
            ("56", ":56"),
            ("->", ":->"),
        ];
        for (raw, literal) in tests {
            assert_eq!(
                Keyword::try_new(raw).map(|kw| kw.to_string()).as_deref(),
                Ok(literal),
                "Keyword::try_new({raw:?})",
            );
        }
    }

    // Other tests:

    #[test]
    fn other_string_cases() {
        let tests = [
            // Non-printable and non-whitespace:
            ("\u{07}", Err(Error::NonPrintableString)),
            ("a\u{07}bc", Err(Error::NonPrintableString)),
            ("\u{7F}", Err(Error::NonPrintableString)),
            ("a\u{7F}bc", Err(Error::NonPrintableString)),
        ];
        for (raw, res) in tests {
            assert_eq!(
                StringLit::try_new(raw).map(|sym| sym.to_string()),
                res,
                "StringLit::try_new({raw:?})",
            );
        }
    }

    #[test]
    fn other_symbol_cases() {
        let tests = [
            // Reserved words:
            ("BINARY", Ok("|BINARY|")),
            ("_", Ok("|_|")),
            // Reserved for solver:
            (".abc", Err(Error::SolverReservedSymbol)),
            ("@abc", Err(Error::SolverReservedSymbol)),
            // But only at the start of a symbol:
            ("a.bc", Ok("a.bc")),
            ("a@bc", Ok("a@bc")),
            // Unquotable:
            ("|", Err(Error::UnquotableSymbol)),
            ("a|bc", Err(Error::UnquotableSymbol)),
            ("\\", Err(Error::UnquotableSymbol)),
            ("a\\bc", Err(Error::UnquotableSymbol)),
            ("\u{07}", Err(Error::NonPrintableSymbol)),
            ("a\u{07}bc", Err(Error::NonPrintableSymbol)),
            ("\u{7F}", Err(Error::NonPrintableSymbol)),
            ("a\u{7F}bc", Err(Error::NonPrintableSymbol)),
        ];
        for (raw, res) in tests {
            assert_eq!(
                Symbol::try_new(raw).map(|sym| sym.to_string()),
                res.map(|s| s.to_owned()),
                "Symbol::try_new({raw:?})",
            );
        }
    }
}
