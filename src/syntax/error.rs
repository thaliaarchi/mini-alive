//! Diagnostics representation and rendering.

use std::{error, fmt, num::ParseIntError};

use crate::syntax::{
    ast::ResolvedVar,
    lex::{Token, TokenSet},
    source::{SourceFile, Span},
};

// TODO:
// - Handle secondary spans for variable errors.

/// An error.
#[derive(Clone, PartialEq, Eq)]
pub struct Error<'s> {
    /// The details of the error.
    pub detail: ErrorDetail<'s>,
    /// The source span of the error.
    pub span: Span,
    /// The source file containing the error.
    pub src: &'s SourceFile,
}

/// The details of an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorDetail<'s> {
    /// A syntax error.
    Syntax(SyntaxError),
    /// A variable reference error.
    Var(VarError<'s>),
}

/// A syntax error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxError {
    /// The kind of error.
    pub kind: SyntaxErrorKind,
    /// The token which caused the error.
    pub tok: Token,
    /// The context in the grammar.
    pub ctx: SyntaxContext,
}

/// A kind of syntax error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyntaxErrorKind {
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

/// Context in the grammar for a syntax error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxContext {
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

/// A variable error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VarError<'s> {
    /// The variable which caused the error.
    pub var: ResolvedVar<'s>,
    /// The kind of error.
    pub kind: VarErrorKind,
}

/// A kind of variable error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VarErrorKind {
    /// Undefined variable.
    Undefined {
        /// The variable kind.
        kind: VarKind,
    },
    /// Refined variable.
    Redefined {
        /// The span of the first definition.
        first_span: Span,
    },
    /// Variable references definition of another kind.
    KindMismatch {
        /// The kind of the variable.
        kind: VarKind,
        /// The kind of the definition.
        def_kind: VarKind,
        /// The span of the definition.
        def_span: Span,
    },
    /// Variable is less than next available ID.
    NonIncreasingNumeric {
        /// The next available ID.
        min: u32,
    },
}

/// The kind of a variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarKind {
    /// Basic block.
    BBlock,
    /// Value (instruction or parameter).
    Value,
}

impl From<SyntaxError> for ErrorDetail<'_> {
    fn from(err: SyntaxError) -> Self {
        ErrorDetail::Syntax(err)
    }
}

impl<'s> From<VarError<'s>> for ErrorDetail<'s> {
    fn from(err: VarError<'s>) -> Self {
        ErrorDetail::Var(err)
    }
}

impl error::Error for Error<'_> {}

impl fmt::Debug for Error<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Error")
                .field("detail", &self.detail)
                .field("span", &self.span)
                .field("src", &self.src)
                .finish()
        } else {
            fmt::Display::fmt(&self, f)
        }
    }
}

impl fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: ")?;
        match &self.detail {
            ErrorDetail::Syntax(err) => {
                write!(f, "{}; found {}", err.kind, err.tok)?;
                if err.tok.can_vary() {
                    write!(f, " `{}`", self.span.text(self.src).escape_debug())?;
                }
            }
            ErrorDetail::Var(err) => write!(f, "{err}")?,
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
        match &self.detail {
            ErrorDetail::Syntax(err) => {
                writeln!(f, "{:>n$} = context: parsing {}", "", err.ctx, n = width)?;
            }
            ErrorDetail::Var(_) => {}
        }
        Ok(())
    }
}

impl fmt::Display for SyntaxErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SyntaxErrorKind::ExpectedToken(mut tokens) => match tokens.len() {
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
            SyntaxErrorKind::ExpectedIdent(ident) => write!(f, "expected `{ident}`"),
            SyntaxErrorKind::TopLevel => write!(f, "invalid start of top-level item"),
            SyntaxErrorKind::Id(ref err) => write!(f, "invalid integer ID: {err}"),
            SyntaxErrorKind::IntLit(ref err) => write!(f, "invalid integer literal: {err}"),
            SyntaxErrorKind::TypeName => write!(f, "unknown type name"),
            SyntaxErrorKind::LitName => write!(f, "unknown literal name"),
            SyntaxErrorKind::UnexpectedResult => {
                write!(f, "unexpected result value on void instruction")
            }
            SyntaxErrorKind::UnsupportedInst => write!(f, "unsupported instruction"),
            SyntaxErrorKind::MissingTerminator => write!(f, "basic block missing terminator"),
            SyntaxErrorKind::Cond => write!(f, "invalid Boolean conditional"),
        }
    }
}

impl fmt::Display for SyntaxContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SyntaxContext::TopLevel => "a top-level item",
            SyntaxContext::Func => "a function",
            SyntaxContext::FuncDeclare => "a function declaration",
            SyntaxContext::BBlock => "a basic block",
            SyntaxContext::Inst => "an instruction",
            SyntaxContext::InstResult => "the result of an instruction",
            SyntaxContext::InstOp => "the opcode of an instruction",
            SyntaxContext::ArithInst => "an arithmetic instruction",
            SyntaxContext::ExtractValueInst => "an `extractvalue` instruction",
            SyntaxContext::InsertValueInst => "an `insertvalue` instruction",
            SyntaxContext::AllocaInst => "an `alloca` instruction",
            SyntaxContext::LoadInst => "a `load` instruction",
            SyntaxContext::StoreInst => "a `store` instruction",
            SyntaxContext::ICmpInst => "an `icmp` instruction",
            SyntaxContext::PhiInst => "a `phi` instruction",
            SyntaxContext::CallInst => "a `call` instruction",
            SyntaxContext::RetInst => "a `ret` instruction",
            SyntaxContext::BrInst => "a `br` instruction",
            SyntaxContext::Cond => "a Boolean conditional",
            SyntaxContext::Val => "a value",
            SyntaxContext::Type => "a type",
            SyntaxContext::StructType => "a struct type",
            SyntaxContext::ArrayType => "an array type",
            SyntaxContext::Lit => "a literal",
            SyntaxContext::StructLit => "a struct literal",
            SyntaxContext::ArrayLit => "an array literal",
        })
    }
}

impl fmt::Display for VarError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let var = &self.var;
        match self.kind {
            VarErrorKind::Undefined { kind } => write!(f, "undefined {kind} %{var}"),
            VarErrorKind::Redefined { .. } => write!(f, "redefined %{var}"),
            VarErrorKind::KindMismatch {
                def_kind: found,
                kind: expected,
                ..
            } => write!(f, "%{var} references {found}, but expected {expected}"),
            VarErrorKind::NonIncreasingNumeric { min } => {
                write!(f, "%{var} is less than the next available ID %{min}")
            }
        }
    }
}

impl fmt::Display for VarKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            VarKind::BBlock => "basic block",
            VarKind::Value => "value",
        })
    }
}
