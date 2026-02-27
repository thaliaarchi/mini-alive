//! An IR for SMT queries, independent of any solver.

use std::{
    fmt,
    ops::{Index, Range},
};

// TODO:
// - Validate sorts on insertion.
// - Fix eq and ite polymorphism.
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

/// A sort without fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKind {
    /// Boolean.
    Bool,
    /// Bit-vector.
    Bv,
}

/// The ID of a term in a context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TermId(u32);

/// An SMT IR expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    /// Boolean.
    Bool(Bool),
    /// Bit-vector.
    Bv(Bv),
}

/// A boolean term.
#[expect(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
#[rustfmt::skip]
pub enum Bool {
    /// Boolean constant.
    Const { value: bool },
    /// AND.
    And { lhs: TermId, rhs: TermId },
    /// OR.
    Or { lhs: TermId, rhs: TermId },
    /// NOT.
    Not { arg: TermId },
    /// Equality.
    Eq { lhs: TermId, rhs: TermId },
    /// If-then-else.
    Ite { cond: TermId, then_: TermId, else_: TermId },
}

/// A bit-vector term.
#[expect(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
#[rustfmt::skip]
pub enum Bv {
    /// Signed bit-vector constant.
    Int64 { value: i64, bits: u32 },
    /// Unsigned bit-vector constant.
    UInt64 { value: u64, bits: u32 },
    /// Addition.
    Add { lhs: TermId, rhs: TermId },
    /// Subtraction.
    Sub { lhs: TermId, rhs: TermId },
    /// Multiplication.
    Mul { lhs: TermId, rhs: TermId },
    /// Signed division.
    SDiv { lhs: TermId, rhs: TermId },
    /// Unsigned division.
    UDiv { lhs: TermId, rhs: TermId },
    /// Signed remainder.
    SRem { lhs: TermId, rhs: TermId },
    /// Unsigned remainder.
    URem { lhs: TermId, rhs: TermId },
    /// Negation.
    Neg { arg: TermId },
    /// Left shift.
    Shl { lhs: TermId, rhs: TermId },
    /// Arithmetic left shift.
    AShr { lhs: TermId, rhs: TermId },
    /// Logical left shift.
    LShr { lhs: TermId, rhs: TermId },
    /// Bitwise AND.
    And { lhs: TermId, rhs: TermId },
    /// Bitwise OR.
    Or { lhs: TermId, rhs: TermId },
    /// Bitwise XOR.
    Xor { lhs: TermId, rhs: TermId },
    /// Bitwise NOT.
    Not { arg: TermId },
    /// Sign extension.
    SignExt { bv: TermId, extend_by: u32 },
    /// Zero extension.
    ZeroExt { bv: TermId, extend_by: u32 },
    /// Concatenation.
    Concat { lhs: TermId, rhs: TermId },
    /// Bit extraction.
    Extract { bv: TermId, bits: Range<u32> },
    /// Signed less-then.
    Sle { lhs: TermId, rhs: TermId },
    /// Unsigned less-than.
    Ule { lhs: TermId, rhs: TermId },
}

impl Context {
    /// Constructs an empty context.
    pub fn new() -> Self {
        Context { terms: Vec::new() }
    }

    /// Inserts a term into the context and returns its ID.
    pub fn insert<T: Into<Term>>(&mut self, term: T) -> TermId {
        self.insert_(term.into())
    }

    fn insert_(&mut self, term: Term) -> TermId {
        let id = TermId(self.terms.len().try_into().unwrap());
        self.terms.push(term);
        id
    }
}

impl TermId {
    /// Gets the index of the term ID.
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

macro_rules! unwrap_sort(($func:ident, $kind:ident) => {
    #[doc = concat!("Unwraps the term as a `", stringify!($kind), "` or panics.")]
    pub fn $func(&self) -> &$kind {
        match self {
            Term::$kind(b) => b,
            _ => panic!(
                "expected {}, but found {}",
                SortKind::$kind,
                self.sort_kind(),
            ),
        }
    }
});

impl Term {
    unwrap_sort!(unwrap_bool, Bool);
    unwrap_sort!(unwrap_bv, Bv);

    /// Gets the sort of the term.
    pub fn sort(&self, ctx: &Context) -> Sort {
        match self {
            Term::Bool(_) => Sort::Bool,
            Term::Bv(bv) => Sort::Bv { bits: bv.bits(ctx) },
        }
    }

    /// Gets the sort kind of the term.
    pub fn sort_kind(&self) -> SortKind {
        match self {
            Term::Bool(_) => SortKind::Bool,
            Term::Bv(_) => SortKind::Bv,
        }
    }
}

impl Bv {
    /// Gets the bit width of this bit-vector.
    pub fn bits(&self, ctx: &Context) -> u32 {
        match *self {
            Bv::Int64 { bits, .. } | Bv::UInt64 { bits, .. } => bits,
            Bv::Add { lhs, .. }
            | Bv::Sub { lhs, .. }
            | Bv::Mul { lhs, .. }
            | Bv::SDiv { lhs, .. }
            | Bv::UDiv { lhs, .. }
            | Bv::SRem { lhs, .. }
            | Bv::URem { lhs, .. }
            | Bv::Neg { arg: lhs }
            | Bv::Shl { lhs, .. }
            | Bv::AShr { lhs, .. }
            | Bv::LShr { lhs, .. }
            | Bv::And { lhs, .. }
            | Bv::Or { lhs, .. }
            | Bv::Xor { lhs, .. }
            | Bv::Not { arg: lhs }
            | Bv::Sle { lhs, .. }
            | Bv::Ule { lhs, .. } => ctx[lhs].unwrap_bv().bits(ctx),
            Bv::SignExt { bv, extend_by } | Bv::ZeroExt { bv, extend_by } => {
                ctx[bv].unwrap_bv().bits(ctx) + extend_by
            }
            Bv::Concat { lhs, rhs } => {
                ctx[lhs].unwrap_bv().bits(ctx) + ctx[rhs].unwrap_bv().bits(ctx)
            }
            Bv::Extract { ref bits, .. } => bits.end - bits.start,
        }
    }
}

impl Index<TermId> for Context {
    type Output = Term;

    fn index(&self, id: TermId) -> &Self::Output {
        &self.terms[id.as_usize()]
    }
}

impl From<Bool> for Term {
    fn from(b: Bool) -> Self {
        Term::Bool(b)
    }
}

impl From<Bv> for Term {
    fn from(bv: Bv) -> Self {
        Term::Bv(bv)
    }
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, term) in self.terms.iter().enumerate() {
            let id = TermId(i as _);
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

impl fmt::Display for SortKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SortKind::Bool => "bool",
            SortKind::Bv => "bv",
        })
    }
}

impl fmt::Display for TermId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.as_usize())
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Bool(b) => b.fmt(f),
            Term::Bv(bv) => bv.fmt(f),
        }
    }
}

impl fmt::Display for Bool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Bool::Const { value } => write!(f, "const {value}"),
            Bool::And { lhs, rhs } => write!(f, "and {lhs}, {rhs}"),
            Bool::Or { lhs, rhs } => write!(f, "or {lhs}, {rhs}"),
            Bool::Not { arg } => write!(f, "not {arg}"),
            Bool::Eq { lhs, rhs } => write!(f, "eq {lhs}, {rhs}"),
            Bool::Ite { cond, then_, else_ } => write!(f, "ite {cond}, {then_}, {else_}"),
        }
    }
}

impl fmt::Display for Bv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Bv::Int64 { value, bits } => write!(f, "int64 {value}, bits={bits}"),
            Bv::UInt64 { value, bits } => write!(f, "uint64 {value}, bits={bits}"),
            Bv::Add { lhs, rhs } => write!(f, "add {lhs}, {rhs}"),
            Bv::Sub { lhs, rhs } => write!(f, "sub {lhs}, {rhs}"),
            Bv::Mul { lhs, rhs } => write!(f, "mul {lhs}, {rhs}"),
            Bv::Neg { arg } => write!(f, "neg {arg}"),
            Bv::SDiv { lhs, rhs } => write!(f, "sdiv {lhs}, {rhs}"),
            Bv::UDiv { lhs, rhs } => write!(f, "udiv {lhs}, {rhs}"),
            Bv::SRem { lhs, rhs } => write!(f, "srem {lhs}, {rhs}"),
            Bv::URem { lhs, rhs } => write!(f, "urem {lhs}, {rhs}"),
            Bv::Shl { lhs, rhs } => write!(f, "shl {lhs}, {rhs}"),
            Bv::AShr { lhs, rhs } => write!(f, "ashr {lhs}, {rhs}"),
            Bv::LShr { lhs, rhs } => write!(f, "lshr {lhs}, {rhs}"),
            Bv::And { lhs, rhs } => write!(f, "and {lhs}, {rhs}"),
            Bv::Or { lhs, rhs } => write!(f, "or {lhs}, {rhs}"),
            Bv::Xor { lhs, rhs } => write!(f, "xor {lhs}, {rhs}"),
            Bv::Not { arg } => write!(f, "not {arg}"),
            Bv::SignExt { bv, extend_by } => write!(f, "signext {bv}, extend_by={extend_by}"),
            Bv::ZeroExt { bv, extend_by } => write!(f, "zeroext {bv}, extend_by={extend_by}"),
            Bv::Concat { lhs, rhs } => write!(f, "concat {lhs}, {rhs}"),
            Bv::Extract { bv, ref bits } => write!(f, "extract {bv}, bits={bits:?}"),
            Bv::Sle { lhs, rhs } => write!(f, "sle {lhs}, {rhs}"),
            Bv::Ule { lhs, rhs } => write!(f, "ule {lhs}, {rhs}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example() {
        let mut ctx = Context::new();
        let b = ctx.insert(Bool::Const { value: false });
        let b = ctx.insert(Bool::Not { arg: b });
        let x = ctx.insert(Bv::Int64 { value: 1, bits: 64 });
        let y = ctx.insert(Bv::Int64 { value: 2, bits: 64 });
        let _z = ctx.insert(Bool::Ite {
            cond: b,
            then_: x,
            else_: y,
        });
        assert_eq!(
            format!("{ctx:?}"),
            "%0 : bool = const false
%1 : bool = not %0
%2 : bv64 = int64 1, bits=64
%3 : bv64 = int64 2, bits=64
%4 : bool = ite %1, %2, %3
"
        );
    }
}
