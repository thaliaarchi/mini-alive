//! Z3 backend for SMT IR.

use std::{ffi::CStr, fmt, ops::Index};

use z3_sys::{ErrorCode, Z3_ast, Z3_context, Z3_get_error_code, Z3_get_error_msg, Z3_sort};

use crate::smt::ir::{Bool, Bv, Sort, TermId};

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

    /// Lowers an SMT IR bool to Z3.
    pub fn lower_bool(&self, b: &Bool) -> Result<Z3_ast, Error> {
        Ok(match *b {
            Bool::Const { value } => {
                if value {
                    cvt!(Z3_mk_true(self.ctx))
                } else {
                    cvt!(Z3_mk_false(self.ctx))
                }
            }
            Bool::And { lhs, rhs } => cvt!(Z3_mk_and(self.ctx, 2, [self[lhs], self[rhs]].as_ptr())),
            Bool::Or { lhs, rhs } => cvt!(Z3_mk_or(self.ctx, 2, [self[lhs], self[rhs]].as_ptr())),
            Bool::Not { arg } => cvt!(Z3_mk_not(self.ctx, self[arg])),
            Bool::Eq { lhs, rhs } => cvt!(Z3_mk_eq(self.ctx, self[lhs], self[rhs])),
            Bool::Ite { cond, then_, else_ } => {
                cvt!(Z3_mk_ite(self.ctx, self[cond], self[then_], self[else_]))
            }
        })
    }

    /// Lowers an SMT IR bit-vector to Z3.
    pub fn lower_bv(&self, bv: &Bv) -> Result<Z3_ast, Error> {
        Ok(match *bv {
            Bv::Int64 { value, bits } => {
                let sort = cvt!(Z3_mk_bv_sort(self.ctx, bits));
                cvt!(Z3_mk_int64(self.ctx, value, sort))
            }
            Bv::UInt64 { value, bits } => {
                let sort = cvt!(Z3_mk_bv_sort(self.ctx, bits));
                cvt!(Z3_mk_unsigned_int64(self.ctx, value, sort))
            }
            Bv::Add { lhs, rhs } => cvt!(Z3_mk_bvadd(self.ctx, self[lhs], self[rhs])),
            Bv::Sub { lhs, rhs } => cvt!(Z3_mk_bvsub(self.ctx, self[lhs], self[rhs])),
            Bv::Mul { lhs, rhs } => cvt!(Z3_mk_bvmul(self.ctx, self[lhs], self[rhs])),
            Bv::SDiv { lhs, rhs } => cvt!(Z3_mk_bvsdiv(self.ctx, self[lhs], self[rhs])),
            Bv::UDiv { lhs, rhs } => cvt!(Z3_mk_bvudiv(self.ctx, self[lhs], self[rhs])),
            Bv::SRem { lhs, rhs } => cvt!(Z3_mk_bvsrem(self.ctx, self[lhs], self[rhs])),
            Bv::URem { lhs, rhs } => cvt!(Z3_mk_bvurem(self.ctx, self[lhs], self[rhs])),
            Bv::Neg { arg } => cvt!(Z3_mk_bvneg(self.ctx, self[arg])),
            Bv::Shl { lhs, rhs } => cvt!(Z3_mk_bvshl(self.ctx, self[lhs], self[rhs])),
            Bv::AShr { lhs, rhs } => cvt!(Z3_mk_bvashr(self.ctx, self[lhs], self[rhs])),
            Bv::LShr { lhs, rhs } => cvt!(Z3_mk_bvlshr(self.ctx, self[lhs], self[rhs])),
            Bv::And { lhs, rhs } => cvt!(Z3_mk_bvand(self.ctx, self[lhs], self[rhs])),
            Bv::Or { lhs, rhs } => cvt!(Z3_mk_bvor(self.ctx, self[lhs], self[rhs])),
            Bv::Xor { lhs, rhs } => cvt!(Z3_mk_bvxor(self.ctx, self[lhs], self[rhs])),
            Bv::Not { arg: bv } => cvt!(Z3_mk_bvnot(self.ctx, self[bv])),
            Bv::SignExt { bv, extend_by } => cvt!(Z3_mk_sign_ext(self.ctx, extend_by, self[bv])),
            Bv::ZeroExt { bv, extend_by } => cvt!(Z3_mk_zero_ext(self.ctx, extend_by, self[bv])),
            Bv::Concat { lhs, rhs } => cvt!(Z3_mk_concat(self.ctx, self[lhs], self[rhs])),
            Bv::Extract { bv, ref bits } => {
                cvt!(Z3_mk_extract(self.ctx, bits.end, bits.start, self[bv]))
            }
            Bv::Sle { lhs, rhs } => cvt!(Z3_mk_bvsle(self.ctx, self[lhs], self[rhs])),
            Bv::Ule { lhs, rhs } => cvt!(Z3_mk_bvule(self.ctx, self[lhs], self[rhs])),
        })
    }

    /// Lowers an SMT IR sort to Z3.
    pub fn lower_sort(&self, sort: Sort) -> Result<Z3_sort, Error> {
        Ok(match sort {
            Sort::Bool => cvt!(Z3_mk_bool_sort(self.ctx)),
            Sort::Bv { bits } => cvt!(Z3_mk_bv_sort(self.ctx, bits)),
        })
    }
}

impl Index<TermId> for Z3Builder {
    type Output = Z3_ast;

    fn index(&self, id: TermId) -> &Self::Output {
        self.terms[id.as_usize()]
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
