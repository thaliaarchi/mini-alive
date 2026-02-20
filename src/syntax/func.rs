//! Control flow syntax.

use std::fmt;

use crate::syntax::{
    inst::Inst,
    value::{GlobalName, LocalName, Type},
};

/// A function: `"define" type global_name params "{" (entry_bb bb*)? "}"`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Func {
    /// The return type.
    pub ret_ty: Type,
    /// The name of the function.
    pub name: GlobalName,
    /// The function parameters.
    pub params: Vec<(Type, LocalName)>,
    /// The basic blocks.
    pub bbs: Vec<BBlock>,
}

/// A basic block: `label? inst* inst_term`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BBlock {
    /// The basic block label.
    pub label: Option<String>,
    /// The instructions in the basic block.
    pub insts: Vec<Inst>,
}

impl fmt::Display for Func {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "define {} {}(", self.ret_ty, self.name)?;
        f.write_str(") {\n")?;
        let mut first = true;
        for bb in &self.bbs {
            if !first {
                f.write_str("\n")?;
            }
            first = false;
            bb.fmt(f)?;
        }
        f.write_str("}\n")
    }
}

impl fmt::Display for BBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(label) = &self.label {
            writeln!(f, "{label}:")?;
        }
        for inst in &self.insts {
            writeln!(f, "    {inst}")?;
        }
        Ok(())
    }
}
