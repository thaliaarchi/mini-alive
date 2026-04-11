//! Z3 backend for SMT IR.

use std::{ffi::CStr, fmt, ops::Index};

use z3_sys::{ErrorCode, Z3_ast, Z3_context, Z3_get_error_code, Z3_get_error_msg};

use crate::smt::ir::{Context, Term, TermId};

/// A builder for producing Z3 queries from SMT IR.
pub struct Z3Builder {
    ctx: Z3_context,
    /// Lowered terms, indexed by `TermId`.
    terms: Vec<Option<Z3_ast>>,
}

/// An error from executing a Z3 function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// The name of the Z3 function which caused the error.
    pub func: &'static str,
    /// The returned error code.
    pub code: ErrorCode,
    /// A message describing the error.
    pub message: String,
}

/// Executes a Z3 function and wraps its errors.
macro_rules! cvt(($func:ident($ctx:expr $(, $arg:expr)* $(,)?)) => {
    match unsafe { z3_sys::$func($ctx, $($arg),*) } {
        Some(res) => res,
        None => return Err(Error::from_context($ctx, stringify!($func))),
    }
});

impl Z3Builder {
    /// Constructs a builder for the Z3 context and a number of terms.
    pub fn new(ctx: Z3_context, terms: usize) -> Self {
        Z3Builder {
            ctx,
            terms: vec![None; terms],
        }
    }

    /// Lowers SMT IR to Z3.
    pub fn lower(&mut self, ctx: &Context) -> Result<(), Error> {
        for id in ctx.iter_ids() {
            self.terms[id.index()] = Some(self.lower_term(&ctx[id])?);
        }
        Ok(())
    }

    /// Lowers an SMT IR term to Z3.
    fn lower_term(&self, term: &Term) -> Result<Z3_ast, Error> {
        Ok(match *term {
            Term::BoolConst { value } => {
                if value {
                    cvt!(Z3_mk_true(self.ctx))
                } else {
                    cvt!(Z3_mk_false(self.ctx))
                }
            }
            Term::BoolAnd { lhs, rhs } => {
                cvt!(Z3_mk_and(self.ctx, 2, [self[lhs], self[rhs]].as_ptr()))
            }
            Term::BoolOr { lhs, rhs } => {
                cvt!(Z3_mk_or(self.ctx, 2, [self[lhs], self[rhs]].as_ptr()))
            }
            Term::BoolNot { arg } => cvt!(Z3_mk_not(self.ctx, self[arg])),
            Term::Eq { lhs, rhs } => cvt!(Z3_mk_eq(self.ctx, self[lhs], self[rhs])),
            Term::Ite { cond, then_, else_ } => {
                cvt!(Z3_mk_ite(self.ctx, self[cond], self[then_], self[else_]))
            }
            Term::BvInt64 { value, bits } => {
                let sort = cvt!(Z3_mk_bv_sort(self.ctx, bits));
                cvt!(Z3_mk_int64(self.ctx, value, sort))
            }
            Term::BvUInt64 { value, bits } => {
                let sort = cvt!(Z3_mk_bv_sort(self.ctx, bits));
                cvt!(Z3_mk_unsigned_int64(self.ctx, value, sort))
            }
            Term::BvAdd { lhs, rhs } => cvt!(Z3_mk_bvadd(self.ctx, self[lhs], self[rhs])),
            Term::BvSub { lhs, rhs } => cvt!(Z3_mk_bvsub(self.ctx, self[lhs], self[rhs])),
            Term::BvMul { lhs, rhs } => cvt!(Z3_mk_bvmul(self.ctx, self[lhs], self[rhs])),
            Term::BvSDiv { lhs, rhs } => cvt!(Z3_mk_bvsdiv(self.ctx, self[lhs], self[rhs])),
            Term::BvUDiv { lhs, rhs } => cvt!(Z3_mk_bvudiv(self.ctx, self[lhs], self[rhs])),
            Term::BvSRem { lhs, rhs } => cvt!(Z3_mk_bvsrem(self.ctx, self[lhs], self[rhs])),
            Term::BvURem { lhs, rhs } => cvt!(Z3_mk_bvurem(self.ctx, self[lhs], self[rhs])),
            Term::BvNeg { arg } => cvt!(Z3_mk_bvneg(self.ctx, self[arg])),
            Term::BvShl { lhs, rhs } => cvt!(Z3_mk_bvshl(self.ctx, self[lhs], self[rhs])),
            Term::BvAShr { lhs, rhs } => cvt!(Z3_mk_bvashr(self.ctx, self[lhs], self[rhs])),
            Term::BvLShr { lhs, rhs } => cvt!(Z3_mk_bvlshr(self.ctx, self[lhs], self[rhs])),
            Term::BvAnd { lhs, rhs } => cvt!(Z3_mk_bvand(self.ctx, self[lhs], self[rhs])),
            Term::BvOr { lhs, rhs } => cvt!(Z3_mk_bvor(self.ctx, self[lhs], self[rhs])),
            Term::BvXor { lhs, rhs } => cvt!(Z3_mk_bvxor(self.ctx, self[lhs], self[rhs])),
            Term::BvNot { arg: bv } => cvt!(Z3_mk_bvnot(self.ctx, self[bv])),
            Term::BvSignExt { bv, extend_by } => {
                cvt!(Z3_mk_sign_ext(self.ctx, extend_by, self[bv]))
            }
            Term::BvZeroExt { bv, extend_by } => {
                cvt!(Z3_mk_zero_ext(self.ctx, extend_by, self[bv]))
            }
            Term::BvConcat { lhs, rhs } => cvt!(Z3_mk_concat(self.ctx, self[lhs], self[rhs])),
            Term::BvExtract { bv, high, low } => {
                cvt!(Z3_mk_extract(self.ctx, high, low, self[bv]))
            }
            Term::BvSle { lhs, rhs } => cvt!(Z3_mk_bvsle(self.ctx, self[lhs], self[rhs])),
            Term::BvUle { lhs, rhs } => cvt!(Z3_mk_bvule(self.ctx, self[lhs], self[rhs])),
        })
    }
}

impl Index<TermId> for Z3Builder {
    type Output = Z3_ast;

    fn index(&self, id: TermId) -> &Self::Output {
        self.terms[id.index()]
            .as_ref()
            .expect("referenced term not lowered")
    }
}

impl Error {
    /// Creates a Z3 error for the last error tracked by the context.
    fn from_context(ctx: Z3_context, func: &'static str) -> Self {
        unsafe {
            let code = Z3_get_error_code(ctx);
            let message = CStr::from_ptr(Z3_get_error_msg(ctx, code))
                .to_string_lossy()
                .into_owned();
            Error {
                func,
                code,
                message,
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error in {}: {}", self.func, self.message)
    }
}
