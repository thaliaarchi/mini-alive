//! Syntax nodes for instructions.

use std::{
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    syntax::ast::{Cond, GlobalVar, LocalVar, Type, TypedVal, Val},
    util::make_enum,
};

// TODO:
// - Deduplicate the result value type in `ExtractValue` and `InsertValue`.

/// An instruction and associated metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstData<'s> {
    /// The instruction.
    pub inst: Inst<'s>,
    /// The name of the SSA value produced by this instruction. Present iff this
    /// is a value instruction.
    pub name: Option<LocalVar<'s>>,
    /// The type of the SSA value produced by this instruction. `Void` if this
    /// is not a value instruction.
    pub ty: Type,
}

/// An instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inst<'s> {
    /// Arithmetic operations
    Arith(Arith<'s>),
    /// `extractvalue`
    ExtractValue(ExtractValue<'s>),
    /// `insertvalue`
    InsertValue(InsertValue<'s>),
    /// `alloca`
    Alloca(Alloca<'s>),
    /// `load`
    Load(Load<'s>),
    /// `store`
    Store(Store<'s>),
    /// `icmp`
    ICmp(ICmp<'s>),
    /// `phi`
    Phi(Phi<'s>),
    /// `call`
    Call(Call<'s>),
    /// `ret`
    Ret(Ret<'s>),
    /// Unconditional `br`
    UncondBr(UncondBr<'s>),
    /// Conditional `br`
    CondBr(CondBr<'s>),
}

/// An instruction opcode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode {
    /// Arithmetic operations
    Arith(ArithOp),
    /// `extractvalue`
    ExtractValue,
    /// `insertvalue`
    InsertValue,
    /// `alloca`
    Alloca,
    /// `load`
    Load,
    /// `store`
    Store,
    /// `icmp`
    ICmp,
    /// `phi`
    Phi,
    /// `call`
    Call,
    /// `ret`
    Ret,
    /// `br`
    Br,
}

make_enum! {
    /// Instruction operation.
    pub enum ArithOp;
    Add => "add",
    Sub => "sub",
    Mul => "mul",
    UDiv => "udiv",
    SDiv => "sdiv",
    URem => "urem",
    SRem => "srem",
    Shl => "shl",
    LShr => "lshr",
    AShr => "ashr",
    And => "and",
    Or => "or",
    Xor => "xor",
}

/// Arithmetic operation: `(local_var "=")? arith int_ty val "," val`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arith<'s> {
    /// The arithmetic operation.
    pub op: ArithOp,
    /// The LHS value.
    pub lhs: Val<'s>,
    /// The RHS value.
    pub rhs: Val<'s>,
}

/// Aggregate element access: `(local_var "=")? "extractvalue" struct_ty val "," int_lit ("," int_lit)*`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractValue<'s> {
    /// The struct.
    pub agg: TypedVal<'s>,
    /// The indices of the element to access.
    pub indices: Vec<usize>,
}

/// Aggregate element write: `(local_var "=")? "insertvalue" struct_ty val "," type val "," int_lit ("," int_lit)*`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsertValue<'s> {
    /// The struct.
    pub agg: TypedVal<'s>,
    /// The value to write to the element.
    pub val: TypedVal<'s>,
    /// The indices of the element to write.
    pub indices: Vec<usize>,
}

/// Stack allocation: `(local_var "=")? "alloca" type ("," int_ty val)?`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alloca<'s> {
    /// The type of the elements.
    pub elem_ty: Type,
    /// The number of elements.
    pub count: Option<usize>,
    /// The lifetime of the source.
    pub lifetime: PhantomData<&'s str>,
}

/// Memory load: `(local_var "=")? "load" type "," ptr_ty val ("," "align" int_lit)?`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Load<'s> {
    /// The address to load from.
    pub ptr: TypedVal<'s>,
    /// The alignment of the operation.
    pub align: Option<usize>,
}

/// Memory store: `"store" type val "," ptr_ty val ("," "align" int_lit)?`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Store<'s> {
    /// The value to store.
    pub val: TypedVal<'s>,
    /// The address to store at.
    pub ptr: TypedVal<'s>,
    /// The alignment of the operation.
    pub align: Option<usize>,
}

/// Integer comparison: `(local_var "=")? "icmp" cond type val "," val`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ICmp<'s> {
    /// The Boolean conditional.
    pub cond: Cond,
    /// The type of the LHS and RHS.
    pub ty: Type,
    /// The LHS value.
    pub lhs: Val<'s>,
    /// The RHS value.
    pub rhs: Val<'s>,
}

/// Phi: `(local_var "=")? "phi" type "[" val "," local_var "]" ("," "[" val "," local_var "]")*`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Phi<'s> {
    /// A value for each predecessor basic block.
    pub sources: Vec<(Val<'s>, LocalVar<'s>)>,
}

/// Function call: `(local_var "=")? "call" type global_var "(" (arg ("," arg)*)? ")"`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call<'s> {
    /// The function to call.
    pub func: GlobalVar<'s>,
    /// The arguments to pass.
    pub args: Vec<TypedVal<'s>>,
}

/// Function return: `"ret" type val`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ret<'s> {
    /// The value to return from the function.
    pub val: TypedVal<'s>,
}

/// Unconditional branch: `"br" "label" local_var`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncondBr<'s> {
    /// The label to jump to.
    pub label: LocalVar<'s>,
}

/// Conditional branch: `"br" bool_ty bool_val "," "label" local_var "," "label" local_var`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CondBr<'s> {
    /// The Boolean condition.
    pub cond: TypedVal<'s>,
    /// The label to jump to if the condition is true.
    pub label_true: LocalVar<'s>,
    /// The label to jump to if the condition is false.
    pub label_false: LocalVar<'s>,
}

impl<'s> Deref for InstData<'s> {
    type Target = Inst<'s>;

    fn deref(&self) -> &Self::Target {
        &self.inst
    }
}
impl<'s> DerefMut for InstData<'s> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inst
    }
}

impl Inst<'_> {
    /// Returns whether the instruction is a basic block terminator.
    pub fn is_terminator(&self) -> bool {
        matches!(self, Inst::Ret(_) | Inst::UncondBr(_) | Inst::CondBr(_))
    }

    /// Returns whether the instruction produces a value.
    pub fn is_value(&self) -> bool {
        matches!(
            self,
            Inst::Arith(_)
                | Inst::ExtractValue(_)
                | Inst::InsertValue(_)
                | Inst::Alloca(_)
                | Inst::Load(_)
                | Inst::ICmp(_)
                | Inst::Phi(_)
                | Inst::Call(_)
        )
    }
}

macro_rules! impl_from_for_inst(($($Ty:ident),* $(,)?) => {
    $(impl<'s> From<$Ty<'s>> for Inst<'s> {
        fn from(inst: $Ty<'s>) -> Self {
            Inst::$Ty(inst)
        }
    })*
});
impl_from_for_inst! {
    Arith,
    ExtractValue,
    InsertValue,
    Alloca,
    Load,
    Store,
    ICmp,
    Phi,
    Call,
    Ret,
    UncondBr,
    CondBr,
}

impl fmt::Display for InstData<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{name} = ")?;
        }
        match &self.inst {
            Inst::Arith(inst) => write!(f, "{} {} {}, {}", inst.op, self.ty, inst.lhs, inst.rhs),
            Inst::ExtractValue(inst) => {
                write!(f, "extractvalue {}", inst.agg)?;
                for &n in &inst.indices {
                    write!(f, ", {n}")?;
                }
                Ok(())
            }
            Inst::InsertValue(inst) => {
                write!(f, "insertvalue {}, {}", inst.agg, inst.val)?;
                for &n in &inst.indices {
                    write!(f, ", {n}")?;
                }
                Ok(())
            }
            Inst::Alloca(inst) => {
                write!(f, "alloca {}", inst.elem_ty)?;
                if let Some(elems) = inst.count {
                    write!(f, ", {elems}")?;
                }
                Ok(())
            }
            Inst::Load(inst) => write!(f, "load {}, {}", self.ty, inst.ptr),
            Inst::Store(inst) => write!(f, "store {}, {}", inst.val, inst.ptr),
            Inst::ICmp(inst) => write!(
                f,
                "icmp {} {} {}, {}",
                inst.cond, inst.ty, inst.lhs, inst.rhs,
            ),
            Inst::Phi(inst) => {
                write!(f, "phi {}", self.ty)?;
                let mut first = true;
                for (val, pred) in &inst.sources {
                    if !first {
                        f.write_str(",")?;
                    }
                    first = false;
                    write!(f, " [ {val}, {pred} ]")?;
                }
                Ok(())
            }
            Inst::Call(inst) => {
                write!(f, "call {} {}(", self.ty, inst.func)?;
                let mut first = true;
                for arg in &inst.args {
                    if !first {
                        f.write_str(", ")?;
                    }
                    first = false;
                    write!(f, "{arg}")?;
                }
                f.write_str(")")
            }
            Inst::Ret(inst) => write!(f, "ret {}", inst.val),
            Inst::UncondBr(inst) => write!(f, "br label {}", inst.label),
            Inst::CondBr(inst) => write!(
                f,
                "br {}, label {}, label {}",
                inst.cond, inst.label_true, inst.label_false,
            ),
        }
    }
}
