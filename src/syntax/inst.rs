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

/// An instruction and associated metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstData<'s> {
    /// The instruction.
    pub inst: Inst<'s>,
    /// The name of the SSA value produced by this instruction. Present iff this
    /// is a value instruction.
    pub name: Option<LocalVar<'s>>,
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
    /// The type of the LHS and RHS.
    pub ty: Type,
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
    /// The type of the allocated elements.
    pub ty: Type,
    /// The number of elements.
    pub count: Option<usize>,
    /// The lifetime of the source.
    pub lifetime: PhantomData<&'s str>,
}

/// Memory load: `(local_var "=")? "load" type "," ptr_ty val ("," "align" int_lit)?`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Load<'s> {
    /// The type to load as.
    pub ty: Type,
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
    /// The type of the value.
    pub ty: Type,
    /// A value for each predecessor basic block.
    pub sources: Vec<(Val<'s>, LocalVar<'s>)>,
}

/// Function call: `(local_var "=")? "call" type global_var "(" (arg ("," arg)*)? ")"`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call<'s> {
    /// The type of the return value.
    pub ret_ty: Type,
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
        self.inst.fmt(f)
    }
}

impl fmt::Display for Inst<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inst: &dyn fmt::Display = match self {
            Inst::Arith(arith) => arith,
            Inst::ExtractValue(extractvalue) => extractvalue,
            Inst::InsertValue(insertvalue) => insertvalue,
            Inst::Alloca(alloca) => alloca,
            Inst::Load(load) => load,
            Inst::Store(store) => store,
            Inst::ICmp(icmp) => icmp,
            Inst::Phi(phi) => phi,
            Inst::Call(call) => call,
            Inst::Ret(ret) => ret,
            Inst::UncondBr(br1) => br1,
            Inst::CondBr(br2) => br2,
        };
        inst.fmt(f)
    }
}

impl fmt::Display for Arith<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}, {}", self.op, self.ty, self.lhs, self.rhs)
    }
}

impl fmt::Display for ExtractValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "extractvalue {}", self.agg)?;
        for &n in &self.indices {
            write!(f, ", {n}")?;
        }
        Ok(())
    }
}

impl fmt::Display for InsertValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "insertvalue {}, {}", self.agg, self.val)?;
        for &n in &self.indices {
            write!(f, ", {n}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Alloca<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "alloca {}", self.ty)?;
        if let Some(elems) = self.count {
            write!(f, ", {elems}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Load<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "load {}, {}", self.ty, self.ptr)
    }
}

impl fmt::Display for Store<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "store {}, {}", self.val, self.ptr)
    }
}

impl fmt::Display for ICmp<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "icmp {} {} {}, {}",
            self.cond, self.ty, self.lhs, self.rhs,
        )
    }
}

impl fmt::Display for Phi<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "phi {}", self.ty)?;
        let mut first = true;
        for (val, pred) in &self.sources {
            if !first {
                f.write_str(",")?;
            }
            first = false;
            write!(f, " [ {val}, {pred} ]")?;
        }
        Ok(())
    }
}

impl fmt::Display for Call<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "call {} {}(", self.ret_ty, self.func)?;
        let mut first = true;
        for arg in &self.args {
            if !first {
                f.write_str(", ")?;
            }
            first = false;
            write!(f, "{arg}")?;
        }
        f.write_str(")")
    }
}

impl fmt::Display for Ret<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ret {}", self.val)
    }
}

impl fmt::Display for UncondBr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "br label {}", self.label)
    }
}

impl fmt::Display for CondBr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "br {}, label {}, label {}",
            self.cond, self.label_true, self.label_false,
        )
    }
}
