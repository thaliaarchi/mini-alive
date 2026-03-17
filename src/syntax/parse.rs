//! Parsing Mini-Alive source.

use std::{cell::Cell, error, ffi::OsStr, fmt, num::ParseIntError, str::FromStr};

use crate::syntax::{
    ast::{BBlock, Cond, Func, GlobalName, Lit, LocalName, Type, TypedVal, Val},
    inst::{
        Alloca, Arith, ArithOp, Call, CondBr, ExtractValue, ICmp, InsertValue, Inst, Load, Phi,
        Ret, Store, UncondBr,
    },
    lex::{Lexeme, Lexer, Token, TokenSet, token_set},
};

// TODO:
// - Parse attributes, but discard them and warn.

/// A parser for Mini-Alive source.
pub struct Parser<'s> {
    lexer: Lexer<'s>,
    peek: Option<Lexeme<'s>>,
    ctx: Cell<Context>,
    filename: String,
}

/// A parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error<'s> {
    /// The lexeme which caused the error.
    pub lex: Lexeme<'s>,
    /// The kind of error.
    pub kind: ErrorKind,
    /// The context in the grammar.
    pub ctx: Context,
    /// The filename of the source.
    pub filename: String,
    /// The line in the source text.
    pub line: &'s str,
}

/// A kind of parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// Expected one of these tokens.
    ExpectedToken(TokenSet),
    /// Expected this identifier.
    ExpectedIdent(&'static str),
    /// Unknown type name.
    TypeName,
    /// Unknown literal name.
    LitName,
    /// Failed to parse integer literal.
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
    /// The top-level.
    TopLevel,
    /// A function.
    Func,
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

/// A drop guard which restores the original context.
///
/// # Safety
///
/// It must never escape the scope in which is is created.
struct ContextGuard {
    ctx: *mut Context,
    old_ctx: Context,
}

impl<'s> Parser<'s> {
    /// Constructs a parser for Mini-Alive source.
    pub fn new<T: AsRef<OsStr>>(src: &'s str, filename: T) -> Self {
        Parser::from_lexer(Lexer::new(src), filename.as_ref())
    }

    /// Constructs a parser from a lexer.
    pub fn from_lexer(lexer: Lexer<'s>, filename: &OsStr) -> Self {
        let filename = filename.to_string_lossy().into_owned();
        Parser {
            lexer,
            peek: None,
            ctx: Cell::new(Context::TopLevel),
            filename,
        }
    }

    /// Returns whether the parser is at EOF.
    pub fn eof(&mut self) -> bool {
        self.peek().tok == Token::Eof
    }

    /// Parses a function.
    pub fn parse_func(&mut self) -> Result<Func, Error<'s>> {
        let _ctx = self.with_ctx(Context::Func);
        self.expect_ident("define")?;
        let ret_ty = self.parse_type()?;
        let name = self.expect_global_name()?;

        let mut params = Vec::new();
        self.expect(Token::LParen)?;
        if self.peek().tok != Token::RParen {
            loop {
                let ty = self.parse_type()?;
                let param_name = self.expect_local_name()?;
                params.push((ty, param_name));
                if self.peek().tok == Token::RParen {
                    break;
                }
                self.expect(Token::Comma)?;
            }
        }
        self.expect(Token::RParen)?;

        let mut bbs = Vec::new();
        self.expect(Token::LBrace)?;
        loop {
            if self.next_if(Token::RBrace).is_some() {
                break;
            }
            bbs.push(self.parse_bb()?);
        }

        Ok(Func {
            ret_ty,
            name,
            params,
            bbs,
        })
    }

    /// Parses a basic block.
    pub(super) fn parse_bb(&mut self) -> Result<BBlock, Error<'s>> {
        let _ctx = self.with_ctx(Context::BBlock);
        let label = self
            .next_if(Token::Label)
            .map(|label| label.text[..label.text.len() - 1].to_owned());
        let mut insts = Vec::new();
        while !token_set!(Label | RBrace).contains(self.peek().tok) {
            insts.push(self.parse_inst()?);
        }
        Ok(BBlock { label, insts })
    }

    /// Parses an instruction.
    pub(super) fn parse_inst(&mut self) -> Result<Inst, Error<'s>> {
        let _ctx = self.with_ctx(Context::Inst);
        let result = self.next_if(Token::LocalName);
        if result.is_some() {
            let _ctx = self.with_ctx(Context::InstResult);
            self.expect(Token::Eq)?;
        }
        let op = {
            let _ctx = self.with_ctx(Context::InstOp);
            self.expect(Token::Ident)?
        };
        if let Some(arith) = ArithOp::from_str(op.text) {
            let result = self.require_result(result, op)?;
            let ty = self.parse_type()?;
            let lhs = self.parse_val()?;
            self.expect(Token::Comma)?;
            let rhs = self.parse_val()?;
            return Ok(Inst::from(Arith {
                result,
                op: arith,
                ty,
                lhs,
                rhs,
            }));
        }
        match op.text {
            "extractvalue" => {
                let _ctx = self.with_ctx(Context::ExtractValueInst);
                let result = self.require_result(result, op)?;
                let agg = self.parse_typed_val()?;
                self.expect(Token::Comma)?;
                let indices = self.parse_indices()?;
                return Ok(Inst::from(ExtractValue {
                    result,
                    agg,
                    indices,
                }));
            }
            "insertvalue" => {
                let _ctx = self.with_ctx(Context::InsertValueInst);
                let result = self.require_result(result, op)?;
                let agg = self.parse_typed_val()?;
                self.expect(Token::Comma)?;
                let val = self.parse_typed_val()?;
                self.expect(Token::Comma)?;
                let indices = self.parse_indices()?;
                Ok(Inst::from(InsertValue {
                    result,
                    agg,
                    val,
                    indices,
                }))
            }
            "alloca" => {
                let _ctx = self.with_ctx(Context::AllocaInst);
                let result = self.require_result(result, op)?;
                let ty = self.parse_type()?;
                let count = if self.next_if(Token::Comma).is_some() {
                    Some(self.expect_int()?)
                } else {
                    None
                };
                Ok(Inst::from(Alloca { result, ty, count }))
            }
            "load" => {
                let _ctx = self.with_ctx(Context::LoadInst);
                let result = self.require_result(result, op)?;
                let ty = self.parse_type()?;
                self.expect(Token::Comma)?;
                let ptr = self.parse_typed_val()?;
                let align = self.parse_align()?;
                Ok(Inst::from(Load {
                    result,
                    ty,
                    ptr,
                    align,
                }))
            }
            "store" => {
                let _ctx = self.with_ctx(Context::StoreInst);
                self.forbid_result(result)?;
                let val = self.parse_typed_val()?;
                self.expect(Token::Comma)?;
                let ptr = self.parse_typed_val()?;
                let align = self.parse_align()?;
                Ok(Inst::from(Store { val, ptr, align }))
            }
            "icmp" => {
                let _ctx = self.with_ctx(Context::ICmpInst);
                let result = self.require_result(result, op)?;
                let cond = self.expect(Token::Ident)?;
                let Some(cond) = Cond::from_str(cond.text) else {
                    return Err(self.err(cond, ErrorKind::Cond, self.ctx()));
                };
                let ty = self.parse_type()?;
                let lhs = self.parse_val()?;
                self.expect(Token::Comma)?;
                let rhs = self.parse_val()?;
                Ok(Inst::from(ICmp {
                    result,
                    cond,
                    ty,
                    lhs,
                    rhs,
                }))
            }
            "phi" => {
                let _ctx = self.with_ctx(Context::PhiInst);
                let result = self.require_result(result, op)?;
                let ty = self.parse_type()?;
                let mut sources = Vec::new();
                loop {
                    self.expect(Token::LBracket)?;
                    let val = self.parse_val()?;
                    self.expect(Token::Comma)?;
                    let label = self.expect_local_name()?;
                    self.expect(Token::RBracket)?;
                    sources.push((val, label));
                    if self.next_if(Token::Comma).is_none() {
                        break;
                    }
                }
                Ok(Inst::from(Phi {
                    result,
                    ty,
                    sources,
                }))
            }
            "call" => {
                let _ctx = self.with_ctx(Context::CallInst);
                let result = self.require_result(result, op)?;
                let ret_ty = self.parse_type()?;
                let func = self.expect_global_name()?;
                self.expect(Token::LParen)?;
                let mut args = Vec::new();
                if self.peek().tok != Token::RParen {
                    loop {
                        args.push(self.parse_typed_val()?);
                        if self.peek().tok == Token::RParen {
                            break;
                        }
                        self.expect(Token::Comma)?;
                    }
                }
                self.expect(Token::RParen)?;
                Ok(Inst::from(Call {
                    result,
                    ret_ty,
                    func,
                    args,
                }))
            }
            "ret" => {
                let _ctx = self.with_ctx(Context::RetInst);
                self.forbid_result(result)?;
                let val = self.parse_typed_val()?;
                Ok(Inst::from(Ret { val }))
            }
            "br" => {
                let _ctx = self.with_ctx(Context::BrInst);
                self.forbid_result(result)?;
                let peek = self.peek();
                if peek.tok == Token::Ident && peek.text == "label" {
                    self.bump();
                    let label = self.expect_local_name()?;
                    Ok(Inst::from(UncondBr { label }))
                } else {
                    let cond = self.parse_typed_val()?;
                    self.expect(Token::Comma)?;
                    self.expect_ident("label")?;
                    let label_true = self.expect_local_name()?;
                    self.expect(Token::Comma)?;
                    self.expect_ident("label")?;
                    let label_false = self.expect_local_name()?;
                    Ok(Inst::from(CondBr {
                        cond,
                        label_true,
                        label_false,
                    }))
                }
            }
            _ => Err(self.err(op, ErrorKind::UnsupportedInst, self.ctx())),
        }
    }

    /// Parses a typed value.
    pub(super) fn parse_typed_val(&mut self) -> Result<TypedVal, Error<'s>> {
        let ty = self.parse_type()?;
        let val = self.parse_val()?;
        Ok(TypedVal { ty, val })
    }

    /// Parses a value.
    pub(super) fn parse_val(&mut self) -> Result<Val, Error<'s>> {
        let _ctx = self.with_ctx(Context::Val);
        if self.peek().tok == Token::LocalName {
            Ok(Val::Local(self.expect_local_name()?))
        } else {
            Ok(Val::Lit(self.parse_lit()?))
        }
    }

    /// Parses a type.
    pub(super) fn parse_type(&mut self) -> Result<Type, Error<'s>> {
        let _ctx = self.with_ctx(Context::Type);
        let first = self.expect(token_set!(Ident | LBrace | LBracket))?;
        let ty = match first.tok {
            Token::Ident => match first.text {
                "i16" => Type::I16,
                "ptr" => Type::Ptr,
                "i1" => Type::Bool,
                _ => return Err(self.err(first, ErrorKind::TypeName, self.ctx())),
            },
            Token::LBrace => {
                let _ctx = self.with_ctx(Context::StructType);
                let mut fields = Vec::new();
                if self.peek().tok != Token::RBrace {
                    loop {
                        fields.push(self.parse_type()?);
                        if self.peek().tok == Token::RBrace {
                            break;
                        }
                        self.expect(Token::Comma)?;
                    }
                }
                self.expect(Token::RBrace)?;
                Type::Struct(fields)
            }
            Token::LBracket => {
                let _ctx = self.with_ctx(Context::ArrayType);
                let n = self.expect_int()?;
                self.expect_ident("x")?;
                let ty = self.parse_type()?;
                self.expect(Token::RBracket)?;
                Type::Array(n, Box::new(ty))
            }
            _ => unreachable!(),
        };
        Ok(ty)
    }

    /// Parses a literal value.
    pub(super) fn parse_lit(&mut self) -> Result<Lit, Error<'s>> {
        let _ctx = self.with_ctx(Context::Lit);
        let first = self.expect(token_set!(Int | Ident | LBrace | LBracket))?;
        let ty = match first.tok {
            Token::Int => Lit::I16(self.parse_int(first)?),
            Token::Ident => match first.text {
                "null" => Lit::Null,
                _ => return Err(self.err(first, ErrorKind::LitName, self.ctx())),
            },
            Token::LBrace => {
                let _ctx = self.with_ctx(Context::StructLit);
                let mut fields = Vec::new();
                if self.peek().tok != Token::RBrace {
                    loop {
                        let ty = self.parse_type()?;
                        let lit = self.parse_lit()?;
                        fields.push((ty, lit));
                        if self.peek().tok == Token::RBrace {
                            break;
                        }
                        self.expect(Token::Comma)?;
                    }
                }
                self.expect(Token::RBrace)?;
                Lit::Struct(fields)
            }
            Token::LBracket => {
                let _ctx = self.with_ctx(Context::ArrayLit);
                let mut elems = Vec::new();
                if self.peek().tok != Token::RBracket {
                    loop {
                        let ty = self.parse_type()?;
                        let lit = self.parse_lit()?;
                        elems.push((ty, lit));
                        if self.peek().tok == Token::RBracket {
                            break;
                        }
                        self.expect(Token::Comma)?;
                    }
                }
                self.expect(Token::RBracket)?;
                Lit::Array(elems)
            }
            _ => unreachable!(),
        };
        Ok(ty)
    }

    /// Parses a sequence of integer indices: `int_lit ("," int_lit)*`
    fn parse_indices(&mut self) -> Result<Vec<usize>, Error<'s>> {
        let mut indices = vec![];
        loop {
            indices.push(self.expect_int()?);
            if self.peek().tok != Token::Comma {
                return Ok(indices);
            }
            self.bump();
        }
    }

    /// Parses an `align` argument for `load` and `store`.
    fn parse_align(&mut self) -> Result<Option<usize>, Error<'s>> {
        if self.next_if(Token::Comma).is_none() {
            return Ok(None);
        }
        self.expect_ident("align")?;
        let align = self.expect_int()?;
        Ok(Some(align))
    }

    fn next(&mut self) -> Lexeme<'s> {
        self.peek.take().unwrap_or_else(|| self.lexer.next())
    }

    fn peek(&mut self) -> &Lexeme<'s> {
        self.peek.get_or_insert_with(|| self.lexer.next())
    }

    fn bump(&mut self) -> Lexeme<'s> {
        self.peek.take().expect("no peek before bump")
    }

    fn next_if(&mut self, expected: impl Into<TokenSet>) -> Option<Lexeme<'s>> {
        let expected = expected.into();
        let lex = self.peek();
        if expected.contains(lex.tok) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn expect(&mut self, expected: impl Into<TokenSet>) -> Result<Lexeme<'s>, Error<'s>> {
        let expected = expected.into();
        let lex = self.next();
        if expected.contains(lex.tok) {
            Ok(lex)
        } else {
            Err(self.err(lex, ErrorKind::ExpectedToken(expected), self.ctx()))
        }
    }

    fn expect_ident(&mut self, ident: &'static str) -> Result<(), Error<'s>> {
        let lex = self.next();
        if lex.tok == Token::Ident && lex.text == ident {
            Ok(())
        } else {
            Err(self.err(lex, ErrorKind::ExpectedIdent(ident), self.ctx()))
        }
    }

    fn expect_global_name(&mut self) -> Result<GlobalName, Error<'s>> {
        let lex = self.expect(Token::GlobalName)?;
        Ok(GlobalName(lex.text[1..].to_owned()))
    }

    fn expect_local_name(&mut self) -> Result<LocalName, Error<'s>> {
        let lex = self.expect(Token::LocalName)?;
        Ok(LocalName(lex.text[1..].to_owned()))
    }

    fn expect_int<T: FromStr<Err = ParseIntError>>(&mut self) -> Result<T, Error<'s>> {
        let lex = self.expect(Token::Int)?;
        self.parse_int(lex)
    }

    fn parse_int<T: FromStr<Err = ParseIntError>>(&self, lex: Lexeme<'s>) -> Result<T, Error<'s>> {
        lex.text
            .parse::<T>()
            .map_err(|err| self.err(lex, ErrorKind::IntLit(err), self.ctx()))
    }

    fn require_result(
        &self,
        result: Option<Lexeme<'s>>,
        op: Lexeme<'s>,
    ) -> Result<LocalName, Error<'s>> {
        match result {
            Some(result) => {
                debug_assert_eq!(result.tok, Token::LocalName);
                Ok(LocalName(result.text[1..].to_owned()))
            }
            None => Err(self.err(op, ErrorKind::MissingResult, self.ctx())),
        }
    }

    fn forbid_result(&self, result: Option<Lexeme<'s>>) -> Result<(), Error<'s>> {
        match result {
            Some(result) => Err(self.err(result, ErrorKind::UnexpectedResult, self.ctx())),
            None => Ok(()),
        }
    }

    fn err(&self, lex: Lexeme<'s>, kind: ErrorKind, ctx: Context) -> Error<'s> {
        let src = self.lexer.src().as_bytes();
        let mut line_start = lex.span.start().offset();
        while line_start != 0 && src[line_start - 1] != b'\n' {
            line_start -= 1;
        }
        let mut line_end = lex.span.end().offset();
        while line_end < src.len() && src[line_end] != b'\n' {
            line_end += 1;
        }
        if line_end != 0 && src[line_end - 1] == b'\r' {
            line_end -= 1;
        }
        Error {
            lex,
            kind,
            ctx,
            filename: self.filename.clone(),
            line: &self.lexer.src()[line_start..line_end],
        }
    }

    /// Gets the current parse context.
    fn ctx(&self) -> Context {
        self.ctx.get()
    }

    /// Sets the current parse context and returns a guard which will reset it
    /// at the end of its scope.
    fn with_ctx(&self, ctx: Context) -> ContextGuard {
        ContextGuard {
            ctx: self.ctx.as_ptr(),
            old_ctx: self.ctx.replace(ctx),
        }
    }
}

/// Resets the parse context to the old context.
impl Drop for ContextGuard {
    fn drop(&mut self) {
        unsafe { *self.ctx = self.old_ctx };
    }
}

impl fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}; found {}", self.kind, self.lex.tok)?;
        if self.lex.tok.can_vary() {
            write!(f, " `{}`", self.lex.text)?;
        }
        writeln!(f)?;
        let span = &self.lex.span;
        debug_assert_eq!(span.start().line(), span.end().line());
        let line_number = span.start().line();
        let width = line_number.ilog10() as usize + 1;
        writeln!(f, "{:>n$}--> {}:{span}", "", self.filename, n = width)?;
        writeln!(f, "{:>n$} |", "", n = width)?;
        writeln!(f, "{line_number} | {}", self.line)?;
        write!(
            f,
            "{:>n$} | {:>start$}{}",
            "",
            "",
            "^".repeat((span.end().column() - span.start().column()).max(1)),
            n = width,
            start = span.start().column() - 1,
        )?;
        writeln!(f)?;
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
            ErrorKind::TypeName => write!(f, "unknown type name"),
            ErrorKind::LitName => write!(f, "unknown literal name"),
            ErrorKind::IntLit(ref err) => write!(f, "failed to parse integer literal: {err}"),
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
            Context::TopLevel => "the top-level",
            Context::Func => "a function",
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
