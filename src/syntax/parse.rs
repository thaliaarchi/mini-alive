//! Parsing Mini-Alive source.

use std::{cell::Cell, num::ParseIntError, str::FromStr};

use crate::syntax::{
    lex::{Lexeme, Lexer, Token, TokenSet, token_set},
    value::{Lit, Type},
};

/// A parser for Mini-Alive source.
pub struct Parser<'s> {
    lexer: Lexer<'s>,
    peek: Option<Lexeme<'s>>,
    ctx: Cell<Context>,
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
    /// Failed to parse an integer literal.
    IntLit(ParseIntError),
}

/// Context in the grammar for a parse error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    /// Top-level.
    TopLevel,
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
    pub fn new(src: &'s str) -> Self {
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

    /// Parses a type.
    pub fn parse_type(&mut self) -> Result<Type, Error<'s>> {
        let _ctx = self.with_ctx(Context::Type);
        let first = self.expect(token_set!(Ident | LBrace | LBracket))?;
        let ty = match first.tok {
            Token::Ident => match first.text {
                "i16" => Type::I16,
                "ptr" => Type::Ptr,
                "i1" => Type::Bool,
                _ => return Err(Error::new(first, ErrorKind::TypeName, self.ctx())),
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
    pub fn parse_lit(&mut self) -> Result<Lit, Error<'s>> {
        let _ctx = self.with_ctx(Context::Lit);
        let first = self.expect(token_set!(Int | Ident | LBrace | LBracket))?;
        let ty = match first.tok {
            Token::Int => Lit::I16(self.parse_int(first)?),
            Token::Ident => match first.text {
                "null" => Lit::Null,
                _ => return Err(Error::new(first, ErrorKind::LitName, self.ctx())),
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

    fn peek(&mut self) -> &Lexeme<'s> {
        self.peek.get_or_insert_with(|| self.lexer.next())
    }

    fn next(&mut self) -> Lexeme<'s> {
        self.peek.take().unwrap_or_else(|| self.lexer.next())
    }

    fn expect(&mut self, expected: impl Into<TokenSet>) -> Result<Lexeme<'s>, Error<'s>> {
        self.expect_(expected.into())
    }

    fn expect_(&mut self, expected: TokenSet) -> Result<Lexeme<'s>, Error<'s>> {
        let lex = self.next();
        if expected.contains(lex.tok) {
            Ok(lex)
        } else {
            Err(Error::new(
                lex,
                ErrorKind::ExpectedToken(expected),
                self.ctx(),
            ))
        }
    }

    fn expect_ident(&mut self, ident: &'static str) -> Result<(), Error<'s>> {
        let lex = self.expect(Token::Ident)?;
        if lex.text == ident {
            Ok(())
        } else {
            Err(Error::new(lex, ErrorKind::ExpectedIdent(ident), self.ctx()))
        }
    }

    fn expect_int<T: FromStr<Err = ParseIntError>>(&mut self) -> Result<T, Error<'s>> {
        let lex = self.expect(Token::Int)?;
        self.parse_int(lex)
    }

    fn parse_int<T: FromStr<Err = ParseIntError>>(&self, lex: Lexeme<'s>) -> Result<T, Error<'s>> {
        lex.text
            .parse::<T>()
            .map_err(|err| Error::new(lex, ErrorKind::IntLit(err), self.ctx()))
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

impl<'s> Error<'s> {
    fn new(lex: Lexeme<'s>, kind: ErrorKind, ctx: Context) -> Self {
        Error { lex, kind, ctx }
    }
}
