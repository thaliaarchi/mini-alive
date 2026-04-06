//! Parsing Mini-Alive source.

use std::{cell::Cell, num::ParseIntError, str::FromStr};

use crate::syntax::{
    ast::{
        BBlock, Cond, Func, FuncProto, GlobalName, Lit, LocalName, Module, TopLevel, Type,
        TypedVal, Val,
    },
    error::{Context, Error, ErrorKind},
    inst::{
        Alloca, Arith, ArithOp, Call, CondBr, ExtractValue, ICmp, InsertValue, Inst, Load, Phi,
        Ret, Store, UncondBr,
    },
    lex::{Lexeme, Lexer, Token, TokenSet, token_set},
    source::SourceFile,
};

// TODO:
// - Parse attributes, but discard them and warn.

/// A parser for Mini-Alive source.
pub struct Parser<'s> {
    lexer: Lexer<'s>,
    peek: Option<Lexeme<'s>>,
    ctx: Cell<Context>,
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
        Parser {
            lexer,
            peek: None,
            ctx: Cell::new(Context::TopLevel),
        }
    }

    /// Returns whether the parser is at EOF.
    pub fn eof(&mut self) -> bool {
        self.peek().tok == Token::Eof
    }

    /// Parses a module.
    pub fn parse_module(&mut self) -> Result<Module, Error<'s>> {
        let mut items = Vec::new();
        while !self.eof() {
            items.push(self.parse_top_level()?);
        }
        Ok(Module { items })
    }

    /// Parses a top-level item.
    fn parse_top_level(&mut self) -> Result<TopLevel, Error<'s>> {
        let _ctx = self.with_ctx(Context::TopLevel);
        let ident = self.expect(Token::Ident)?;
        match ident.text {
            "define" => Ok(TopLevel::Func(self.parse_func()?)),
            "declare" => Ok(TopLevel::FuncDeclare(self.parse_func_declare()?)),
            _ => Err(self.err(ident, ErrorKind::TopLevel)),
        }
    }

    /// Parses a function definition, starting after `"define"`.
    fn parse_func(&mut self) -> Result<Func, Error<'s>> {
        let _ctx = self.with_ctx(Context::Func);
        let proto = self.parse_func_proto()?;
        let mut bbs = Vec::new();
        self.expect(Token::LBrace)?;
        loop {
            if self.next_if(Token::RBrace).is_some() {
                break;
            }
            bbs.push(self.parse_bb()?);
        }
        Ok(Func { proto, bbs })
    }

    /// Parses a function declaration, starting after `"declare"`.
    fn parse_func_declare(&mut self) -> Result<FuncProto, Error<'s>> {
        let _ctx = self.with_ctx(Context::FuncDeclare);
        Ok(self.parse_func_proto()?)
    }

    /// Parses a function prototype.
    fn parse_func_proto(&mut self) -> Result<FuncProto, Error<'s>> {
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

        Ok(FuncProto {
            ret_ty,
            name,
            params,
        })
    }

    /// Parses a basic block.
    fn parse_bb(&mut self) -> Result<BBlock, Error<'s>> {
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
                    return Err(self.err(cond, ErrorKind::Cond));
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
            _ => Err(self.err(op, ErrorKind::UnsupportedInst)),
        }
    }

    /// Parses a typed value.
    fn parse_typed_val(&mut self) -> Result<TypedVal, Error<'s>> {
        let ty = self.parse_type()?;
        let val = self.parse_val()?;
        Ok(TypedVal { ty, val })
    }

    /// Parses a value.
    fn parse_val(&mut self) -> Result<Val, Error<'s>> {
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
            .map_err(|err| self.err(lex, ErrorKind::IntLit(err)))
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
            None => Err(self.err(op, ErrorKind::MissingResult)),
        }
    }

    fn forbid_result(&self, result: Option<Lexeme<'s>>) -> Result<(), Error<'s>> {
        match result {
            Some(result) => Err(self.err(result, ErrorKind::UnexpectedResult)),
            None => Ok(()),
        }
    }

    fn err(&self, lex: Lexeme<'s>, kind: ErrorKind) -> Error<'s> {
        Error {
            lex,
            kind,
            ctx: self.ctx.get(),
            src: self.lexer.src(),
        }
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
