//! An IR for SMT queries, independent of any solver.

use std::{
    fmt,
    ops::{Index, Range},
};

use crate::util::make_id;

// TODO:
// - Validate sorts on insertion.
// - Should sorts be stored with terms? If they're interned, that would make
//   lowering faster.
// - Should and/or be variable arity like SMT-LIB2 and Z3?

/// A manager for SMT IR.
pub struct Context {
    terms: Vec<Term>,
}

/// The sort (type) of a term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sort {
    /// Boolean.
    Bool,
    /// Bit-vector.
    Bv {
        /// Bit width.
        bits: u32,
    },
}

make_id! {
    /// The ID of a term in a context.
    pub struct TermId(..);
    iter TermIdIter "term IDs";
}

/// An SMT IR expression.
#[expect(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
#[rustfmt::skip]
pub enum Term {
    /// Boolean constant.
    BoolConst { value: bool },
    /// AND.
    BoolAnd { lhs: TermId, rhs: TermId },
    /// OR.
    BoolOr { lhs: TermId, rhs: TermId },
    /// NOT.
    BoolNot { arg: TermId },

    /// Equality.
    Eq { lhs: TermId, rhs: TermId },
    /// If-then-else.
    Ite { cond: TermId, then_: TermId, else_: TermId },

    /// Signed bit-vector constant.
    BvInt64 { value: i64, bits: u32 },
    /// Unsigned bit-vector constant.
    BvUInt64 { value: u64, bits: u32 },
    /// Addition.
    BvAdd { lhs: TermId, rhs: TermId },
    /// Subtraction.
    BvSub { lhs: TermId, rhs: TermId },
    /// Multiplication.
    BvMul { lhs: TermId, rhs: TermId },
    /// Signed division.
    BvSDiv { lhs: TermId, rhs: TermId },
    /// Unsigned division.
    BvUDiv { lhs: TermId, rhs: TermId },
    /// Signed remainder.
    BvSRem { lhs: TermId, rhs: TermId },
    /// Unsigned remainder.
    BvURem { lhs: TermId, rhs: TermId },
    /// Negation.
    BvNeg { arg: TermId },
    /// Left shift.
    BvShl { lhs: TermId, rhs: TermId },
    /// Arithmetic left shift.
    BvAShr { lhs: TermId, rhs: TermId },
    /// Logical left shift.
    BvLShr { lhs: TermId, rhs: TermId },
    /// Bitwise AND.
    BvAnd { lhs: TermId, rhs: TermId },
    /// Bitwise OR.
    BvOr { lhs: TermId, rhs: TermId },
    /// Bitwise XOR.
    BvXor { lhs: TermId, rhs: TermId },
    /// Bitwise NOT.
    BvNot { arg: TermId },
    /// Sign extension.
    BvSignExt { bv: TermId, extend_by: u32 },
    /// Zero extension.
    BvZeroExt { bv: TermId, extend_by: u32 },
    /// Concatenation.
    BvConcat { lhs: TermId, rhs: TermId },
    /// Bit extraction.
    BvExtract { bv: TermId, bits: Range<u32> },
    /// Signed less-then.
    BvSle { lhs: TermId, rhs: TermId },
    /// Unsigned less-than.
    BvUle { lhs: TermId, rhs: TermId },
}

impl Context {
    /// Constructs an empty context.
    pub fn new() -> Self {
        Context { terms: Vec::new() }
    }

    /// Inserts a term into the context and returns its ID.
    pub fn insert(&mut self, term: Term) -> TermId {
        self.terms.push(term);
        assert!(u32::try_from(self.terms.len()).is_ok(), "TermId overflow");
        TermId(self.terms.len() as u32 - 1)
    }

    /// Gets the sort of a term.
    pub fn sort(&self, id: TermId) -> Sort {
        self[id].sort(self)
    }

    /// Returns an iterator over the IDs of the terms in the context.
    pub fn term_ids(&self) -> TermIdIter {
        TermId::iter(TermId(0)..TermId(self.terms.len() as _))
    }
}

impl Sort {
    /// Gets the bit width, if it is a bit-vector.
    pub fn unwrap_bits(&self) -> u32 {
        match *self {
            Sort::Bv { bits } => bits,
            _ => panic!("expected bit-vector, but found {self}"),
        }
    }
}

impl Term {
    /// Gets the sort of the term.
    pub fn sort(&self, ctx: &Context) -> Sort {
        match *self {
            Term::BoolConst { .. }
            | Term::BoolAnd { .. }
            | Term::BoolOr { .. }
            | Term::BoolNot { .. } => Sort::Bool,
            Term::BvInt64 { bits, .. } | Term::BvUInt64 { bits, .. } => Sort::Bv { bits },
            Term::Eq { lhs, .. }
            | Term::Ite { then_: lhs, .. }
            | Term::BvAdd { lhs, .. }
            | Term::BvSub { lhs, .. }
            | Term::BvMul { lhs, .. }
            | Term::BvSDiv { lhs, .. }
            | Term::BvUDiv { lhs, .. }
            | Term::BvSRem { lhs, .. }
            | Term::BvURem { lhs, .. }
            | Term::BvNeg { arg: lhs }
            | Term::BvShl { lhs, .. }
            | Term::BvAShr { lhs, .. }
            | Term::BvLShr { lhs, .. }
            | Term::BvAnd { lhs, .. }
            | Term::BvOr { lhs, .. }
            | Term::BvXor { lhs, .. }
            | Term::BvNot { arg: lhs }
            | Term::BvSle { lhs, .. }
            | Term::BvUle { lhs, .. } => ctx.sort(lhs),
            Term::BvSignExt { bv, extend_by } | Term::BvZeroExt { bv, extend_by } => {
                let bits = ctx.sort(bv).unwrap_bits() + extend_by;
                Sort::Bv { bits }
            }
            Term::BvConcat { lhs, rhs } => {
                let bits = ctx.sort(lhs).unwrap_bits() + ctx.sort(rhs).unwrap_bits();
                Sort::Bv { bits }
            }
            Term::BvExtract { ref bits, .. } => {
                let bits = bits.end - bits.start;
                Sort::Bv { bits }
            }
        }
    }
}

impl Index<TermId> for Context {
    type Output = Term;

    fn index(&self, id: TermId) -> &Self::Output {
        &self.terms[id.as_usize()]
    }
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, term) in self.terms.iter().enumerate() {
            let id = TermId(i as u32);
            let sort = term.sort(self);
            writeln!(f, "{id} : {sort} = {term}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sort::Bool => f.write_str("bool"),
            Sort::Bv { bits } => write!(f, "bv{bits}"),
        }
    }
}

impl fmt::Display for TermId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.as_usize())
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Term::BoolConst { value } => write!(f, "{value}"),
            Term::BoolAnd { lhs, rhs } => write!(f, "and {lhs}, {rhs}"),
            Term::BoolOr { lhs, rhs } => write!(f, "or {lhs}, {rhs}"),
            Term::BoolNot { arg } => write!(f, "not {arg}"),
            Term::Eq { lhs, rhs } => write!(f, "eq {lhs}, {rhs}"),
            Term::Ite { cond, then_, else_ } => write!(f, "ite {cond}, {then_}, {else_}"),
            Term::BvInt64 { value, bits } => write!(f, "int64 {value}, bits={bits}"),
            Term::BvUInt64 { value, bits } => write!(f, "uint64 {value}, bits={bits}"),
            Term::BvAdd { lhs, rhs } => write!(f, "add {lhs}, {rhs}"),
            Term::BvSub { lhs, rhs } => write!(f, "sub {lhs}, {rhs}"),
            Term::BvMul { lhs, rhs } => write!(f, "mul {lhs}, {rhs}"),
            Term::BvNeg { arg } => write!(f, "neg {arg}"),
            Term::BvSDiv { lhs, rhs } => write!(f, "sdiv {lhs}, {rhs}"),
            Term::BvUDiv { lhs, rhs } => write!(f, "udiv {lhs}, {rhs}"),
            Term::BvSRem { lhs, rhs } => write!(f, "srem {lhs}, {rhs}"),
            Term::BvURem { lhs, rhs } => write!(f, "urem {lhs}, {rhs}"),
            Term::BvShl { lhs, rhs } => write!(f, "shl {lhs}, {rhs}"),
            Term::BvAShr { lhs, rhs } => write!(f, "ashr {lhs}, {rhs}"),
            Term::BvLShr { lhs, rhs } => write!(f, "lshr {lhs}, {rhs}"),
            Term::BvAnd { lhs, rhs } => write!(f, "and {lhs}, {rhs}"),
            Term::BvOr { lhs, rhs } => write!(f, "or {lhs}, {rhs}"),
            Term::BvXor { lhs, rhs } => write!(f, "xor {lhs}, {rhs}"),
            Term::BvNot { arg } => write!(f, "not {arg}"),
            Term::BvSignExt { bv, extend_by } => write!(f, "signext {bv}, extend_by={extend_by}"),
            Term::BvZeroExt { bv, extend_by } => write!(f, "zeroext {bv}, extend_by={extend_by}"),
            Term::BvConcat { lhs, rhs } => write!(f, "concat {lhs}, {rhs}"),
            Term::BvExtract { bv, ref bits } => write!(f, "extract {bv}, bits={bits:?}"),
            Term::BvSle { lhs, rhs } => write!(f, "sle {lhs}, {rhs}"),
            Term::BvUle { lhs, rhs } => write!(f, "ule {lhs}, {rhs}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example() {
        let mut ctx = Context::new();
        let b = ctx.insert(Term::BoolConst { value: false });
        let b = ctx.insert(Term::BoolNot { arg: b });
        let x = ctx.insert(Term::BvInt64 { value: 1, bits: 8 });
        let y = ctx.insert(Term::BvInt64 { value: 2, bits: 8 });
        let _z = ctx.insert(Term::Ite {
            cond: b,
            then_: x,
            else_: y,
        });
        assert_eq!(
            format!("{ctx:?}"),
            "\
%0 : bool = false
%1 : bool = not %0
%2 : bv8 = int64 1, bits=8
%3 : bv8 = int64 2, bits=8
%4 : bv8 = ite %1, %2, %3
"
        );
    }
}
