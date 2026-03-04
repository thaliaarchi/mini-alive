//! Lowering from SMT IR to SMT-LIB S-expressions.

use crate::smt::{
    ir::{Context, Sort, Term, TermId},
    smtlib::{Atom, CommandName, List, ListStyle, Reserved, SExp, Script, Symbol},
};

/// Lowers all terms in a context to `define-fun` commands.
pub fn lower_context(ctx: &Context) -> Script {
    let mut script = Script::new();
    for id in ctx.term_ids() {
        script.push(list(
            ListStyle::Hanging,
            vec![
                SExp::from(Atom::CommandName(CommandName::DefineFun)),
                sym(format!("t{}", id.as_usize())),
                list(ListStyle::Vertical, Vec::new()),
                lower_sort(&ctx.sort(id)),
                lower_term(ctx, id),
            ],
        ));
    }
    script
}

fn lower_sort(sort: &Sort) -> SExp {
    match *sort {
        Sort::Bool => sym("Bool"),
        Sort::Bv { bits } => list(
            ListStyle::Hanging,
            vec![Reserved::Underscore.into(), sym("BitVec"), num(bits as i64)],
        ),
    }
}

fn lower_term(ctx: &Context, id: TermId) -> SExp {
    match ctx[id] {
        Term::BoolConst { value } => sym(if value { "true" } else { "false" }),
        Term::BoolAnd { lhs, rhs } => call2("and", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BoolOr { lhs, rhs } => call2("or", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BoolNot { arg } => call1("not", lower_term(ctx, arg)),
        Term::Eq { lhs, rhs } => call2("=", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::Ite { cond, then_, else_ } => call3(
            "ite",
            lower_term(ctx, cond),
            lower_term(ctx, then_),
            lower_term(ctx, else_),
        ),
        Term::BvInt64 { value, bits } => lower_bv_const(value as u64, bits),
        Term::BvUInt64 { value, bits } => lower_bv_const(value, bits),
        Term::BvAdd { lhs, rhs } => call2("bvadd", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvSub { lhs, rhs } => call2("bvsub", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvMul { lhs, rhs } => call2("bvmul", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvSDiv { lhs, rhs } => call2("bvsdiv", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvUDiv { lhs, rhs } => call2("bvudiv", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvSRem { lhs, rhs } => call2("bvsrem", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvURem { lhs, rhs } => call2("bvurem", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvNeg { arg } => call1("bvneg", lower_term(ctx, arg)),
        Term::BvShl { lhs, rhs } => call2("bvshl", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvAShr { lhs, rhs } => call2("bvashr", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvLShr { lhs, rhs } => call2("bvlshr", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvAnd { lhs, rhs } => call2("bvand", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvOr { lhs, rhs } => call2("bvor", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvXor { lhs, rhs } => call2("bvxor", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvNot { arg } => call1("bvnot", lower_term(ctx, arg)),
        Term::BvSignExt { bv, extend_by } => {
            call1_indexed("sign_extend", extend_by, lower_term(ctx, bv))
        }
        Term::BvZeroExt { bv, extend_by } => {
            call1_indexed("zero_extend", extend_by, lower_term(ctx, bv))
        }
        Term::BvConcat { lhs, rhs } => call2("concat", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvExtract { bv, high, low } => {
            call1_indexed2("extract", high as i64, low as i64, lower_term(ctx, bv))
        }
        Term::BvSle { lhs, rhs } => call2("bvsle", lower_term(ctx, lhs), lower_term(ctx, rhs)),
        Term::BvUle { lhs, rhs } => call2("bvule", lower_term(ctx, lhs), lower_term(ctx, rhs)),
    }
}

fn lower_bv_const(value: u64, bits: u32) -> SExp {
    let value = if bits >= 64 {
        value
    } else {
        value & ((1u64 << bits) - 1)
    };
    list(
        ListStyle::Hanging,
        vec![
            Reserved::Underscore.into(),
            sym(format!("bv{value}")),
            num(bits as i64),
        ],
    )
}

fn sym<T: Into<String>>(s: T) -> SExp {
    SExp::from(Symbol::new(s))
}
fn num(n: i64) -> SExp {
    SExp::from(Atom::Numeral(n))
}

fn call1(name: &str, arg: SExp) -> SExp {
    list(ListStyle::Hanging, vec![sym(name), arg])
}
fn call2(name: &str, lhs: SExp, rhs: SExp) -> SExp {
    list(ListStyle::Hanging, vec![sym(name), lhs, rhs])
}
fn call3(name: &str, x: SExp, y: SExp, z: SExp) -> SExp {
    list(ListStyle::Hanging, vec![sym(name), x, y, z])
}
fn call1_indexed(name: &str, idx: u32, arg: SExp) -> SExp {
    list(
        ListStyle::Hanging,
        vec![
            list(
                ListStyle::Hanging,
                vec![Reserved::Underscore.into(), sym(name), num(idx as i64)],
            ),
            arg,
        ],
    )
}
fn call1_indexed2(name: &str, hi: i64, lo: i64, arg: SExp) -> SExp {
    list(
        ListStyle::Hanging,
        vec![
            list(
                ListStyle::Hanging,
                vec![Reserved::Underscore.into(), sym(name), num(hi), num(lo)],
            ),
            arg,
        ],
    )
}

fn list(style: ListStyle, elems: Vec<SExp>) -> SExp {
    SExp::from(List { elems, style })
}

#[cfg(test)]
mod tests {
    use crate::smt::ir::{Context, Term};

    use super::*;

    #[test]
    fn lower_example() {
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
        let script = lower_context(&ctx);
        assert_eq!(
            script.pretty(),
            "\
(define-fun t0 () Bool false)
(define-fun t1 () Bool (not false))
(define-fun t2 () (_ BitVec 8) (_ bv1 8))
(define-fun t3 () (_ BitVec 8) (_ bv2 8))
(define-fun t4
            ()
            (_ BitVec 8)
            (ite (not false) (_ bv1 8) (_ bv2 8)))
"
        );
    }
}
