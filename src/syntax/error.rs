//! Diagnostics representation and rendering.

use std::{error, fmt, num::ParseIntError};

use crate::syntax::{
    lex::{Token, TokenSet},
    source::{SourceFile, Span},
};

/// An error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error<'s> {
    /// The details of the error.
    pub detail: ErrorDetail,
    /// The source span of the error.
    pub span: Span,
    /// The source file containing the error.
    pub src: &'s SourceFile,
}

/// The details of an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorDetail {
    /// A parse error.
    Parse(ParseError),
}

/// A parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The kind of error.
    pub kind: ParseErrorKind,
    /// The token which caused the error.
    pub tok: Token,
    /// The context in the grammar.
    pub ctx: ParseContext,
}

/// A kind of parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Expected one of these tokens.
    ExpectedToken(TokenSet),
    /// Expected this identifier.
    ExpectedIdent(&'static str),
    /// Invalid start of top-level item.
    TopLevel,
    /// Invalid integer ID.
    Id(ParseIntError),
    /// Invalid integer literal.
    IntLit(ParseIntError),
    /// Unknown type name.
    TypeName,
    /// Unknown literal name.
    LitName,
    /// Unexpected result value on void instruction.
    UnexpectedResult,
    /// Unknown instruction.
    UnsupportedInst,
    /// Basic block missing terminator.
    MissingTerminator,
    /// Invalid Boolean conditional.
    Cond,
}

/// Context in the grammar for a parse error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseContext {
    /// A top-level item.
    TopLevel,
    /// A function.
    Func,
    /// A function declaration.
    FuncDeclare,
    /// A basic block.
    BBlock,
    /// An instruction.
    Inst,
    /// The result of an instruction.
    InstResult,
    /// The opcode of an instruction.
    InstOp,
    /// An arithmetic instruction.
    ArithInst,
    /// An `extractvalue` instruction.
    ExtractValueInst,
    /// An `insertvalue` instruction.
    InsertValueInst,
    /// An `alloca` instruction.
    AllocaInst,
    /// A `load` instruction.
    LoadInst,
    /// A `store` instruction.
    StoreInst,
    /// An `icmp` instruction.
    ICmpInst,
    /// A `phi` instruction.
    PhiInst,
    /// A `call` instruction.
    CallInst,
    /// A `ret` instruction.
    RetInst,
    /// A `br` instruction.
    BrInst,
    /// A Boolean conditional.
    Cond,
    /// A value.
    Val,
    /// A type.
    Type,
    /// A struct type.
    StructType,
    /// An array type.
    ArrayType,
    /// A literal.
    Lit,
    /// A struct literal.
    StructLit,
    /// An array literal.
    ArrayLit,
}

impl From<ParseError> for ErrorDetail {
    fn from(err: ParseError) -> Self {
        ErrorDetail::Parse(err)
    }
}

impl fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ErrorDetail::Parse(err) = &self.detail;
        write!(f, "Error: {}; found {}", err.kind, err.tok)?;
        if err.tok.can_vary() {
            write!(f, " `{}`", self.span.text(self.src).escape_debug())?;
        }
        writeln!(f)?;
        let start = self.span.start_position(self.src);
        let end = self.span.end_position(self.src);
        let width = end.line.ilog10() as usize + 1;
        write!(
            f,
            "{:>n$}--> {}:{}:{}",
            "",
            self.src.filename().to_string_lossy(),
            start.line,
            start.column,
            n = width,
        )?;
        if end.column != start.column + 1 {
            write!(f, "-{}:{}", end.line, end.column)?;
        }
        writeln!(f)?;
        writeln!(f, "{:>n$} |", "", n = width)?;
        for line_number in start.line..=end.line {
            let line = self.src.line_text(line_number);
            writeln!(f, "{line_number} | {line}")?;
            let highlight_start = if line_number == start.line {
                start.column - 1
            } else {
                0
            };
            let highlight_end = if line_number == end.line {
                end.column - 1
            } else {
                line.chars().count()
            };
            let highlight = "^".repeat((highlight_end - highlight_start).max(1));
            writeln!(
                f,
                "{:>n$} | {:>highlight_start$}{highlight}",
                "",
                "",
                n = width,
                highlight_start = highlight_start,
            )?;
        }
        writeln!(f, "{:>n$} |", "", n = width)?;
        writeln!(f, "{:>n$} = context: parsing {}", "", err.ctx, n = width)
    }
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ParseErrorKind::ExpectedToken(mut tokens) => match tokens.len() {
                0 => write!(f, "unexpected token"),
                1 => write!(f, "expected {}", tokens.next().unwrap()),
                2 => write!(
                    f,
                    "expected {} or {}",
                    tokens.next().unwrap(),
                    tokens.next().unwrap(),
                ),
                _ => {
                    write!(f, "expected")?;
                    while let Some(tok) = tokens.next() {
                        if tokens.is_empty() {
                            write!(f, " or {tok}")?;
                        } else {
                            write!(f, " {tok},")?;
                        }
                    }
                    Ok(())
                }
            },
            ParseErrorKind::ExpectedIdent(ident) => write!(f, "expected `{ident}`"),
            ParseErrorKind::TopLevel => write!(f, "invalid start of top-level item"),
            ParseErrorKind::Id(ref err) => write!(f, "invalid integer ID: {err}"),
            ParseErrorKind::IntLit(ref err) => write!(f, "invalid integer literal: {err}"),
            ParseErrorKind::TypeName => write!(f, "unknown type name"),
            ParseErrorKind::LitName => write!(f, "unknown literal name"),
            ParseErrorKind::UnexpectedResult => {
                write!(f, "unexpected result value on void instruction")
            }
            ParseErrorKind::UnsupportedInst => write!(f, "unsupported instruction"),
            ParseErrorKind::MissingTerminator => write!(f, "basic block missing terminator"),
            ParseErrorKind::Cond => write!(f, "invalid Boolean conditional"),
        }
    }
}

impl fmt::Display for ParseContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ParseContext::TopLevel => "a top-level item",
            ParseContext::Func => "a function",
            ParseContext::FuncDeclare => "a function declaration",
            ParseContext::BBlock => "a basic block",
            ParseContext::Inst => "an instruction",
            ParseContext::InstResult => "the result of an instruction",
            ParseContext::InstOp => "the opcode of an instruction",
            ParseContext::ArithInst => "an arithmetic instruction",
            ParseContext::ExtractValueInst => "an `extractvalue` instruction",
            ParseContext::InsertValueInst => "an `insertvalue` instruction",
            ParseContext::AllocaInst => "an `alloca` instruction",
            ParseContext::LoadInst => "a `load` instruction",
            ParseContext::StoreInst => "a `store` instruction",
            ParseContext::ICmpInst => "an `icmp` instruction",
            ParseContext::PhiInst => "a `phi` instruction",
            ParseContext::CallInst => "a `call` instruction",
            ParseContext::RetInst => "a `ret` instruction",
            ParseContext::BrInst => "a `br` instruction",
            ParseContext::Cond => "a Boolean conditional",
            ParseContext::Val => "a value",
            ParseContext::Type => "a type",
            ParseContext::StructType => "a struct type",
            ParseContext::ArrayType => "an array type",
            ParseContext::Lit => "a literal",
            ParseContext::StructLit => "a struct literal",
            ParseContext::ArrayLit => "an array literal",
        })
    }
}

impl error::Error for Error<'_> {}
