//! Parsing Mini-Alive source.

use std::{cell::Cell, marker::PhantomData, num::ParseIntError, str::FromStr};

use crate::syntax::{
    ast::{
        BBlock, Cond, Func, FuncProto, GlobalVar, Lit, LocalVar, Module, TopLevel, Type, TypedVal,
        Val, Var,
    },
    build::FuncBuilder,
    error::{Error, SyntaxContext as Context, SyntaxError, SyntaxErrorKind as ErrorKind},
    inst::{
        Alloca, Arith, ArithOp, Call, CondBr, ExtractValue, ICmp, InsertValue, Inst, InstData,
        Load, Phi, Ret, Store, UncondBr,
    },
    lex::{Lexeme, Lexer, Token, TokenSet, token_set},
    source::{SourceFile, Span},
};

// TODO:
// - Improve spans for errors regarding unnamed variables.
// - Parse attributes, but discard them and warn.

/// A parser for Mini-Alive source.
pub struct Parser<'s> {
    lexer: Lexer<'s>,
    peek: Option<Lexeme<'s>>,
    ctx: Cell<Context>,
    builder: FuncBuilder<'s>,
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
    pub fn new(src: &'s SourceFile) -> Self {
        Parser::from_lexer(Lexer::new(src))
    }

    /// Constructs a parser from a lexer.
    pub fn from_lexer(lexer: Lexer<'s>) -> Self {
        let builder = FuncBuilder::new(lexer.src());
        Parser {
            lexer,
            peek: None,
            ctx: Cell::new(Context::TopLevel),
            builder,
        }
    }

    /// Returns whether the parser is at EOF.
    pub fn eof(&mut self) -> bool {
        self.peek().tok == Token::Eof
    }

    /// Parses a module.
    pub fn parse_module(&mut self) -> Result<Module<'s>, Error<'s>> {
        let mut items = Vec::new();
        while !self.eof() {
            items.push(self.parse_top_level()?);
        }
        Ok(Module { items })
    }

    /// Parses a top-level item.
    fn parse_top_level(&mut self) -> Result<TopLevel<'s>, Error<'s>> {
        let _ctx = self.with_ctx(Context::TopLevel);
        let ident = self.expect(Token::Ident)?;
        match ident.text {
            "define" => Ok(TopLevel::Func(self.parse_func()?)),
            "declare" => Ok(TopLevel::FuncDeclare(self.parse_func_declare()?)),
            _ => Err(self.err(ident, ErrorKind::TopLevel)),
        }
    }

    /// Parses a function definition, starting after `"define"`.
    fn parse_func(&mut self) -> Result<Func<'s>, Error<'s>> {
        self.builder.reset();
        let _ctx = self.with_ctx(Context::Func);
        let proto = self.parse_func_proto()?;
        let mut bbs = Vec::new();
        self.expect(Token::LBrace)?;
        if self.peek().tok == Token::RBrace {
            let _ctx = self.with_ctx(Context::BBlock);
            let lex = self.next();
            return Err(self.err(lex, ErrorKind::MissingTerminator));
        }
        while self.next_if(Token::RBrace).is_none() {
            let _ctx = self.with_ctx(Context::BBlock);
            let label = self.next_if(Token::Label);
            let label = self.parse_label(label)?;
            let mut insts = Vec::new();
            loop {
                if token_set!(Label | RBrace | Eof).contains(self.peek().tok) {
                    let lex = self.next();
                    return Err(self.err(lex, ErrorKind::MissingTerminator));
                }
                let inst = self.parse_inst()?;
                let is_terminator = inst.is_terminator();
                insts.push(self.builder.insert_inst(inst));
                if is_terminator {
                    break;
                }
            }
            bbs.push(BBlock { label, insts });
        }
        let arena = self.builder.finish()?;
        Ok(Func { proto, bbs, arena })
    }

    /// Parses a function declaration, starting after `"declare"`.
    fn parse_func_declare(&mut self) -> Result<FuncProto<'s>, Error<'s>> {
        self.builder.reset();
        let _ctx = self.with_ctx(Context::FuncDeclare);
        self.parse_func_proto()
    }

    /// Parses a function prototype.
    fn parse_func_proto(&mut self) -> Result<FuncProto<'s>, Error<'s>> {
        let ret_ty = self.parse_type()?;
        let name = self.expect_global_var()?;

        let mut params = Vec::new();
        self.expect(Token::LParen)?;
        if self.peek().tok != Token::RParen {
            loop {
                let ty_span = self.peek().span;
                let ty = self.parse_type()?;
                let (var, param_span) = if token_set!(Comma | RParen).contains(self.peek().tok) {
                    (Var::Unnamed, ty_span)
                } else {
                    self.expect_local_var()?
                };
                let var = self.builder.define_value(var, param_span)?;
                params.push((ty, var));
                if self.peek().tok == Token::RParen {
                    break;
                }
                self.expect(Token::Comma)?;
            }
        }
        self.expect(Token::RParen)?;

        Ok(FuncProto {
            ret_ty,
            name,
            params,
        })
    }

    /// Parses an instruction.
    pub(super) fn parse_inst(&mut self) -> Result<InstData<'s>, Error<'s>> {
        let _ctx = self.with_ctx(Context::Inst);
        let result = self.next_if(Token::LocalVar);
        if result.is_some() {
            let _ctx = self.with_ctx(Context::InstResult);
            self.expect(Token::Eq)?;
        }
        let op = {
            let _ctx = self.with_ctx(Context::InstOp);
            self.expect(Token::Ident)?
        };
        let mut ty = Type::Void;
        let inst = match op.text {
            _ if let Some(arith) = ArithOp::from_str(op.text) => {
                let _ctx = self.with_ctx(Context::ArithInst);
                ty = self.parse_type()?;
                let lhs = self.parse_val()?;
                self.expect(Token::Comma)?;
                let rhs = self.parse_val()?;
                Inst::from(Arith {
                    op: arith,
                    lhs,
                    rhs,
                })
            }
            "extractvalue" => {
                let _ctx = self.with_ctx(Context::ExtractValueInst);
                let agg = self.parse_typed_val()?;
                ty = agg.ty.clone();
                self.expect(Token::Comma)?;
                let indices = self.parse_indices()?;
                Inst::from(ExtractValue { agg, indices })
            }
            "insertvalue" => {
                let _ctx = self.with_ctx(Context::InsertValueInst);
                let agg = self.parse_typed_val()?;
                ty = agg.ty.clone();
                self.expect(Token::Comma)?;
                let val = self.parse_typed_val()?;
                self.expect(Token::Comma)?;
                let indices = self.parse_indices()?;
                Inst::from(InsertValue { agg, val, indices })
            }
            "alloca" => {
                let _ctx = self.with_ctx(Context::AllocaInst);
                ty = Type::Ptr;
                let elem_ty = self.parse_type()?;
                let count = if self.next_if(Token::Comma).is_some() {
                    Some(self.expect_int()?)
                } else {
                    None
                };
                Inst::from(Alloca {
                    elem_ty,
                    count,
                    lifetime: PhantomData,
                })
            }
            "load" => {
                let _ctx = self.with_ctx(Context::LoadInst);
                ty = self.parse_type()?;
                self.expect(Token::Comma)?;
                let ptr = self.parse_typed_val()?;
                let align = self.parse_align()?;
                Inst::from(Load { ptr, align })
            }
            "store" => {
                let _ctx = self.with_ctx(Context::StoreInst);
                let val = self.parse_typed_val()?;
                self.expect(Token::Comma)?;
                let ptr = self.parse_typed_val()?;
                let align = self.parse_align()?;
                Inst::from(Store { val, ptr, align })
            }
            "icmp" => {
                let _ctx = self.with_ctx(Context::ICmpInst);
                ty = Type::Bool;
                let cond = self.expect(Token::Ident)?;
                let Some(cond) = Cond::from_str(cond.text) else {
                    return Err(self.err(cond, ErrorKind::Cond));
                };
                let ty = self.parse_type()?;
                let lhs = self.parse_val()?;
                self.expect(Token::Comma)?;
                let rhs = self.parse_val()?;
                Inst::from(ICmp { cond, ty, lhs, rhs })
            }
            "phi" => {
                let _ctx = self.with_ctx(Context::PhiInst);
                ty = self.parse_type()?;
                let mut sources = Vec::new();
                loop {
                    self.expect(Token::LBracket)?;
                    let val = self.parse_val()?;
                    self.expect(Token::Comma)?;
                    let label = self.expect_label_var()?;
                    self.expect(Token::RBracket)?;
                    sources.push((val, label));
                    if self.next_if(Token::Comma).is_none() {
                        break;
                    }
                }
                Inst::from(Phi { sources })
            }
            "call" => {
                let _ctx = self.with_ctx(Context::CallInst);
                ty = self.parse_type()?;
                let func = self.expect_global_var()?;
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
                Inst::from(Call { func, args })
            }
            "ret" => {
                let _ctx = self.with_ctx(Context::RetInst);
                let val = self.parse_typed_val()?;
                Inst::from(Ret { val })
            }
            "br" => {
                let _ctx = self.with_ctx(Context::BrInst);
                let peek = self.peek();
                if peek.tok == Token::Ident && peek.text == "label" {
                    self.bump();
                    let label = self.expect_label_var()?;
                    Inst::from(UncondBr { label })
                } else {
                    let cond = self.parse_typed_val()?;
                    self.expect(Token::Comma)?;
                    self.expect_ident("label")?;
                    let label_true = self.expect_label_var()?;
                    self.expect(Token::Comma)?;
                    self.expect_ident("label")?;
                    let label_false = self.expect_label_var()?;
                    Inst::from(CondBr {
                        cond,
                        label_true,
                        label_false,
                    })
                }
            }
            _ => return Err(self.err(op, ErrorKind::UnsupportedInst)),
        };
        let result = if inst.is_value() {
            let (result, span) = match result {
                Some(result) => {
                    let span = result.span;
                    (self.parse_var(&result.text[1..], result)?, span)
                }
                None => (Var::Unnamed, op.span),
            };
            Some(self.builder.define_value(result, span)?)
        } else {
            if let Some(result) = result {
                return Err(self.err(result, ErrorKind::UnexpectedResult));
            }
            None
        };
        Ok(InstData {
            inst,
            name: result,
            ty,
        })
    }

    /// Parses a typed value.
    fn parse_typed_val(&mut self) -> Result<TypedVal<'s>, Error<'s>> {
        let ty = self.parse_type()?;
        let val = self.parse_val()?;
        Ok(TypedVal { ty, val })
    }

    /// Parses a value.
    fn parse_val(&mut self) -> Result<Val<'s>, Error<'s>> {
        let _ctx = self.with_ctx(Context::Val);
        if self.peek().tok == Token::LocalVar {
            Ok(Val::Local(self.expect_value_var()?))
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
                _ => return Err(self.err(first, ErrorKind::TypeName)),
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
                _ => return Err(self.err(first, ErrorKind::LitName)),
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
            Err(self.err(lex, ErrorKind::ExpectedToken(expected)))
        }
    }

    fn expect_ident(&mut self, ident: &'static str) -> Result<(), Error<'s>> {
        let lex = self.next();
        if lex.tok == Token::Ident && lex.text == ident {
            Ok(())
        } else {
            Err(self.err(lex, ErrorKind::ExpectedIdent(ident)))
        }
    }

    fn expect_global_var(&mut self) -> Result<GlobalVar<'s>, Error<'s>> {
        let lex = self.expect(Token::GlobalVar)?;
        Ok(GlobalVar(self.parse_var(&lex.text[1..], lex)?))
    }

    fn expect_local_var(&mut self) -> Result<(Var<'s>, Span), Error<'s>> {
        let lex = self.expect(Token::LocalVar)?;
        let span = lex.span;
        Ok((self.parse_var(&lex.text[1..], lex)?, span))
    }

    fn expect_value_var(&mut self) -> Result<LocalVar<'s>, Error<'s>> {
        let (var, span) = self.expect_local_var()?;
        self.builder.use_value(var, span)
    }

    fn expect_label_var(&mut self) -> Result<LocalVar<'s>, Error<'s>> {
        let (var, span) = self.expect_local_var()?;
        self.builder.use_bblock(var, span)
    }

    fn parse_var(&self, text: &'s str, lex: Lexeme<'s>) -> Result<Var<'s>, Error<'s>> {
        if text.as_bytes()[0].is_ascii_digit() {
            text.parse()
                .map(Var::Numeric)
                .map_err(|err| self.err(lex, ErrorKind::Id(err)))
        } else {
            Ok(Var::Name(text))
        }
    }

    fn parse_label(&mut self, label: Option<Lexeme<'s>>) -> Result<LocalVar<'s>, Error<'s>> {
        let (label, span) = match label {
            Some(lex) => {
                debug_assert_eq!(lex.tok, Token::Label);
                let span = lex.span;
                (self.parse_var(&lex.text[..lex.text.len() - 1], lex)?, span)
            }
            None => (Var::Unnamed, self.peek().span),
        };
        self.builder.define_bblock(label, span)
    }

    fn expect_int<T: FromStr<Err = ParseIntError>>(&mut self) -> Result<T, Error<'s>> {
        let lex = self.expect(Token::Int)?;
        self.parse_int(lex)
    }

    fn parse_int<T: FromStr<Err = ParseIntError>>(&self, lex: Lexeme<'s>) -> Result<T, Error<'s>> {
        lex.text
            .parse::<T>()
            .map_err(|err| self.err(lex, ErrorKind::IntLit(err)))
    }

    /// Resets the function builder.
    #[cfg(test)]
    pub(super) fn reset_builder(&mut self) {
        self.builder.reset();
    }

    fn err(&self, lex: Lexeme<'s>, kind: ErrorKind) -> Error<'s> {
        let err = SyntaxError {
            kind,
            tok: lex.tok,
            ctx: self.ctx.get(),
        };
        Error {
            detail: err.into(),
            span: lex.span,
            src: self.lexer.src(),
        }
    }

    /// Sets the current syntax context and returns a guard which will reset it
    /// at the end of its scope.
    fn with_ctx(&self, ctx: Context) -> ContextGuard {
        ContextGuard {
            ctx: self.ctx.as_ptr(),
            old_ctx: self.ctx.replace(ctx),
        }
    }
}

/// Resets the syntax context to the old context.
impl Drop for ContextGuard {
    fn drop(&mut self) {
        unsafe { *self.ctx = self.old_ctx };
    }
}
