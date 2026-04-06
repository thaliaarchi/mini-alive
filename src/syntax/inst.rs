//! Syntax nodes for instructions.

use std::fmt;

use crate::{
    syntax::ast::{Cond, GlobalName, LocalName, Type, TypedVal, Val},
    util::make_enum,
};

/// An instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inst {
    /// Arithmetic operations
    Arith(Arith),
    /// `extractvalue`
    ExtractValue(ExtractValue),
    /// `insertvalue`
    InsertValue(InsertValue),
    /// `alloca`
    Alloca(Alloca),
    /// `load`
    Load(Load),
    /// `store`
    Store(Store),
    /// `icmp`
    ICmp(ICmp),
    /// `phi`
    Phi(Phi),
    /// `call`
    Call(Call),
    /// `ret`
    Ret(Ret),
    /// Unconditional `br`
    UncondBr(UncondBr),
    /// Conditional `br`
    CondBr(CondBr),
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

/// Common accesses for instructions.
pub trait InstData {
    /// Returns the SSA value name of the result, if this instruction produces a
    /// value.
    fn result(&self) -> Option<&LocalName>;
}

/// Arithmetic operation: `local_name "=" arith int_ty val "," val`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arith {
    /// The result SSA value name.
    pub result: LocalName,
    /// The arithmetic operation.
    pub op: ArithOp,
    /// The type of the LHS and RHS.
    pub ty: Type,
    /// The LHS value.
    pub lhs: Val,
    /// The RHS value.
    pub rhs: Val,
}

/// Aggregate element access: `local_name "=" "extractvalue" struct_ty val "," int_lit ("," int_lit)*`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractValue {
    /// The result SSA value name.
    pub result: LocalName,
    /// The struct.
    pub agg: TypedVal,
    /// The indices of the element to access.
    pub indices: Vec<usize>,
}

/// Aggregate element write: `local_name "=" "insertvalue" struct_ty val "," type val "," int_lit ("," int_lit)*`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsertValue {
    /// The result SSA value name.
    pub result: LocalName,
    /// The struct.
    pub agg: TypedVal,
    /// The value to write to the element.
    pub val: TypedVal,
    /// The indices of the element to write.
    pub indices: Vec<usize>,
}

/// Stack allocation: `local_name "=" "alloca" type ("," int_ty val)?`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alloca {
    /// The result SSA value name.
    pub result: LocalName,
    /// The type of the allocated elements.
    pub ty: Type,
    /// The number of elements.
    pub count: Option<usize>,
}

/// Memory load: `local_name "=" "load" type "," ptr_ty val`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Load {
    /// The result SSA value name.
    pub result: LocalName,
    /// The type to load as.
    pub ty: Type,
    /// The address to load from.
    pub ptr: TypedVal,
    /// The alignment of the operation.
    pub align: Option<usize>,
}

/// Memory store: `"store" type val "," ptr_ty val`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Store {
    /// The value to store.
    pub val: TypedVal,
    /// The address to store at.
    pub ptr: TypedVal,
    /// The alignment of the operation.
    pub align: Option<usize>,
}

/// Integer comparison: `local_name "=" "icmp" cond type val "," val`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ICmp {
    /// The result SSA value name.
    pub result: LocalName,
    /// The Boolean conditional.
    pub cond: Cond,
    /// The type of the LHS and RHS.
    pub ty: Type,
    /// The LHS value.
    pub lhs: Val,
    /// The RHS value.
    pub rhs: Val,
}

/// Phi: `local_name "=" "phi" type "[" val "," local_name "]" ("," "[" val "," local_name "]")*`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Phi {
    /// The result SSA value name.
    pub result: LocalName,
    /// The type of the value.
    pub ty: Type,
    /// A value for each predecessor basic block.
    pub sources: Vec<(Val, LocalName)>,
}

/// Function call: `local_name "=" "call" type global_name "(" (arg ("," arg)*)? ")"`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    /// The result SSA value name.
    pub result: LocalName,
    /// The type of the return value.
    pub ret_ty: Type,
    /// The function to call.
    pub func: GlobalName,
    /// The arguments to pass.
    pub args: Vec<TypedVal>,
}

/// Function return: `"ret" type val`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ret {
    /// The value to return from the function.
    pub val: TypedVal,
}

/// Unconditional branch: `"br" "label" local_name`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncondBr {
    /// The label to jump to.
    pub label: LocalName,
}

/// Conditional branch: `"br" bool_ty bool_val "," "label" local_name "," "label" local_name`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CondBr {
    /// The Boolean condition.
    pub cond: TypedVal,
    /// The label to jump to if the condition is true.
    pub label_true: LocalName,
    /// The label to jump to if the condition is false.
    pub label_false: LocalName,
}

macro_rules! impl_from_for_inst(($($Ty:ident),* $(,)?) => {
    $(impl From<$Ty> for Inst {
        fn from(inst: $Ty) -> Self {
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

impl InstData for Arith {
    fn result(&self) -> Option<&LocalName> {
        Some(&self.result)
    }
}
impl InstData for ExtractValue {
    fn result(&self) -> Option<&LocalName> {
        Some(&self.result)
    }
}
impl InstData for InsertValue {
    fn result(&self) -> Option<&LocalName> {
        Some(&self.result)
    }
}
impl InstData for Alloca {
    fn result(&self) -> Option<&LocalName> {
        Some(&self.result)
    }
}
impl InstData for Load {
    fn result(&self) -> Option<&LocalName> {
        Some(&self.result)
    }
}
impl InstData for Store {
    fn result(&self) -> Option<&LocalName> {
        None
    }
}
impl InstData for ICmp {
    fn result(&self) -> Option<&LocalName> {
        Some(&self.result)
    }
}
impl InstData for Phi {
    fn result(&self) -> Option<&LocalName> {
        Some(&self.result)
    }
}
impl InstData for Call {
    fn result(&self) -> Option<&LocalName> {
        Some(&self.result)
    }
}
impl InstData for Ret {
    fn result(&self) -> Option<&LocalName> {
        None
    }
}
impl InstData for UncondBr {
    fn result(&self) -> Option<&LocalName> {
        None
    }
}
impl InstData for CondBr {
    fn result(&self) -> Option<&LocalName> {
        None
    }
}

impl fmt::Display for Inst {
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

impl fmt::Display for Arith {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} = {} {} {}, {}",
            self.result, self.op, self.ty, self.lhs, self.rhs,
        )
    }
}

impl fmt::Display for ExtractValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} = extractvalue {}", self.result, self.agg)?;
        for &n in &self.indices {
            write!(f, ", {n}")?;
        }
        Ok(())
    }
}

impl fmt::Display for InsertValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} = insertvalue {}, {}",
            self.result, self.agg, self.val,
        )?;
        for &n in &self.indices {
            write!(f, ", {n}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Alloca {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} = alloca {}", self.result, self.ty)?;
        if let Some(elems) = self.count {
            write!(f, ", {elems}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Load {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} = load {}, {}", self.result, self.ty, self.ptr)
    }
}

impl fmt::Display for Store {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "store {}, {}", self.val, self.ptr)
    }
}

impl fmt::Display for ICmp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} = icmp {} {} {}, {}",
            self.result, self.cond, self.ty, self.lhs, self.rhs,
        )
    }
}

impl fmt::Display for Phi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} = phi {}", self.result, self.ty)?;
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

impl fmt::Display for Call {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} = call {} {}(", self.result, self.ret_ty, self.func)?;
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

impl fmt::Display for Ret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ret {}", self.val)
    }
}

impl fmt::Display for UncondBr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "br label {}", self.label)
    }
}

impl fmt::Display for CondBr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "br {}, label {}, label {}",
            self.cond, self.label_true, self.label_false,
        )
    }
}
