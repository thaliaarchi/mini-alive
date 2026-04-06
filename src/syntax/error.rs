//! Diagnostics representation and rendering.

use std::{error, fmt, num::ParseIntError};

use crate::syntax::{
    lex::{Lexeme, TokenSet},
    source::SourceFile,
};

/// A parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error<'s> {
    /// The lexeme which caused the error.
    pub lex: Lexeme<'s>,
    /// The kind of error.
    pub kind: ErrorKind,
    /// The context in the grammar.
    pub ctx: Context,
    /// The source file containing the error.
    pub src: &'s SourceFile,
}

/// A kind of parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// Expected one of these tokens.
    ExpectedToken(TokenSet),
    /// Expected this identifier.
    ExpectedIdent(&'static str),
    /// Invalid start of top-level item.
    TopLevel,
    /// Unknown type name.
    TypeName,
    /// Unknown literal name.
    LitName,
    /// Invalid integer literal.
    IntLit(ParseIntError),
    /// Instruction missing required result value.
    MissingResult,
    /// Unexpected result value on void instruction.
    UnexpectedResult,
    /// Unknown instruction.
    UnsupportedInst,
    /// Invalid Boolean conditional.
    Cond,
}

/// Context in the grammar for a parse error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
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

impl fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}; found {}", self.kind, self.lex.tok)?;
        if self.lex.tok.can_vary() {
            write!(f, " `{}`", self.lex.text)?;
        }
        writeln!(f)?;
        let span = &self.lex.span;
        let start = span.start_position(self.src);
        let end = span.end_position(self.src);
        debug_assert_eq!(start.line, end.line);
        let line_number = start.line;
        let width = line_number.ilog10() as usize + 1;
        write!(
            f,
            "{:>n$}--> {}:{}:{}",
            "",
            self.src.filename().to_string_lossy(),
            start.line,
            start.column,
            n = width
        )?;
        if end.column != start.column + 1 {
            write!(f, "-{}:{}", end.line, end.column)?;
        }
        writeln!(f)?;
        writeln!(f, "{:>n$} |", "", n = width)?;
        writeln!(f, "{line_number} | {}", self.src.line_text(line_number))?;
        writeln!(
            f,
            "{:>n$} | {:>start$}{}",
            "",
            "",
            "^".repeat((end.column - start.column).max(1)),
            n = width,
            start = start.column - 1,
        )?;
        writeln!(f, "{:>n$} |", "", n = width)?;
        writeln!(f, "{:>n$} = context: parsing {}", "", self.ctx, n = width)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ErrorKind::ExpectedToken(mut tokens) => match tokens.len() {
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
            ErrorKind::ExpectedIdent(ident) => write!(f, "expected `{ident}`"),
            ErrorKind::TopLevel => write!(f, "invalid start of top-level item"),
            ErrorKind::TypeName => write!(f, "unknown type name"),
            ErrorKind::LitName => write!(f, "unknown literal name"),
            ErrorKind::IntLit(ref err) => write!(f, "invalid integer literal: {err}"),
            ErrorKind::MissingResult => write!(f, "instruction missing required result value"),
            ErrorKind::UnexpectedResult => write!(f, "unexpected result value on void instruction"),
            ErrorKind::UnsupportedInst => write!(f, "unsupported instruction"),
            ErrorKind::Cond => write!(f, "invalid Boolean conditional"),
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Context::TopLevel => "a top-level item",
            Context::Func => "a function",
            Context::FuncDeclare => "a function declaration",
            Context::BBlock => "a basic block",
            Context::Inst => "an instruction",
            Context::InstResult => "the result of an instruction",
            Context::InstOp => "the opcode of an instruction",
            Context::ArithInst => "an arithmetic instruction",
            Context::ExtractValueInst => "an `extractvalue` instruction",
            Context::InsertValueInst => "an `insertvalue` instruction",
            Context::AllocaInst => "an `alloca` instruction",
            Context::LoadInst => "a `load` instruction",
            Context::StoreInst => "a `store` instruction",
            Context::ICmpInst => "an `icmp` instruction",
            Context::PhiInst => "a `phi` instruction",
            Context::CallInst => "a `call` instruction",
            Context::RetInst => "a `ret` instruction",
            Context::BrInst => "a `br` instruction",
            Context::Cond => "a Boolean conditional",
            Context::Val => "a value",
            Context::Type => "a type",
            Context::StructType => "a struct type",
            Context::ArrayType => "an array type",
            Context::Lit => "a literal",
            Context::StructLit => "a struct literal",
            Context::ArrayLit => "an array literal",
        })
    }
}

impl error::Error for Error<'_> {}
