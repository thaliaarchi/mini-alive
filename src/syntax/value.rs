//! Mini-Alive syntax nodes.

use std::fmt;

// TODO:
// - Implement type checking: it needs unification for 0-element arrays and
//   boolean literals.

/// An instruction: `result op args`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inst {
    /// Result SSA value: `(local_name "=")?`
    pub result: Option<LocalName>,
    /// Instruction name: `ident`
    pub op: String,
    /// Arguments to the instruction: `(arg ("," arg)*)*`
    pub args: Vec<Arg>,
}

/// An argument to an instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg {
    /// Integer literal: `lit`
    Int(usize),
    /// Type: `type`
    Type(Type),
    /// Value: `type val`
    Value(Type, Val),
    /// Label: `"label" local_name`
    Label(LocalName),
    /// Boolean conditional: `cond type val`
    Cond(Cond, Type, Val),
    /// Phi: `type "[" val "," local_name "]" ("," "[" val "," local_name "]")*`
    Phi(Type, Vec<(Val, LocalName)>),
    /// Function call: `type global_name "(" (arg ("," arg)*)? ")"`
    Call(Type, GlobalName, Vec<Arg>),
}

/// A global name (`@`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalName(pub String);

/// A local name (`%`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalName(pub String);

/// A value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Val {
    /// Literal value.
    Lit(Lit),
    /// Local value.
    Local(LocalName),
}

/// A type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    /// 16-bit integer: `"i16"`
    I16,
    /// Pointer: `"ptr"`
    Ptr,
    /// Structure: `"{" (type ("," type)*)? "}"`
    Struct(Vec<Type>),
    /// Array: `"[" int_lit "x" type "]"`
    Array(usize, Box<Type>),
    /// Boolean: `i1`
    Bool,
}

/// A literal value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lit {
    /// 16-bit integer: `-?[0-9]+`
    I16(i16),
    /// Null pointer: `"null"`
    Null,
    /// Structure: `"{" (type lit ("," type lit)*)? "}"`
    Struct(Vec<(Type, Lit)>),
    /// Array: `"[" (type lit ("," type lit)*)? "]"`
    Array(Vec<(Type, Lit)>),
    /// Boolean: `"0" | "1"`
    Bool(bool),
}

/// Boolean conditional.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cond {
    /// `eq`
    Eq,
    /// `ne`
    Ne,
    /// `ugt`
    Ugt,
    /// `uge`
    Uge,
    /// `ult`
    Ult,
    /// `ule`
    Ule,
    /// `sgt`
    Sgt,
    /// `sge`
    Sge,
    /// `slt`
    Slt,
    /// `sle`
    Sle,
}

impl Lit {
    /// Gets the type of this literal value, if it is not ambiguous.
    pub fn ty(&self) -> Option<Type> {
        match self {
            Lit::I16(_) => Some(Type::I16),
            Lit::Null => Some(Type::Ptr),
            Lit::Struct(fields) => Some(Type::Struct(
                fields.iter().map(|(ty, _)| ty.clone()).collect(),
            )),
            Lit::Array(elems) => {
                if let Some((ty, _)) = elems.first() {
                    Some(Type::Array(elems.len(), Box::new(ty.clone())))
                } else {
                    None
                }
            }
            Lit::Bool(_) => Some(Type::Bool),
        }
    }

    /// Checks whether this literal value has the given type.
    pub fn has_type(&self, ty: &Type) -> bool {
        match (self, ty) {
            (Lit::I16(_), Type::I16) => true,
            (Lit::Null, Type::Ptr) => true,
            (Lit::Struct(fields), Type::Struct(types)) => {
                fields.len() == types.len()
                    && fields
                        .iter()
                        .zip(types)
                        .all(|((lit_ty, _), ty)| lit_ty == ty)
            }
            (Lit::Array(elems), Type::Array(n, ty)) => {
                elems.len() == *n && elems.first().is_none_or(|(first_ty, _)| first_ty == &**ty)
            }
            (Lit::Bool(_), Type::Bool) => true,
            _ => false,
        }
    }

    /// Checks whether this literal value is valid for its types.
    pub fn valid(&self) -> bool {
        match self {
            Lit::I16(_) | Lit::Null | Lit::Bool(_) => true,
            Lit::Struct(fields) => fields.iter().all(|(ty, field)| field.has_type(ty)),
            Lit::Array(elems) => elems
                .first()
                .is_none_or(|(ty, _)| elems.iter().all(|(_, lit)| lit.has_type(ty))),
        }
    }
}

impl fmt::Display for Inst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(result) = &self.result {
            write!(f, "{result} = ")?;
        }
        write!(f, "{}", self.op)?;
        let mut first = true;
        for arg in &self.args {
            if !first {
                f.write_str(", ")?;
            }
            first = false;
            arg.fmt(f)?;
        }
        Ok(())
    }
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arg::Int(n) => write!(f, "{n}"),
            Arg::Type(ty) => ty.fmt(f),
            Arg::Value(ty, val) => write!(f, "{ty} {val}"),
            Arg::Label(label) => write!(f, "label {label}"),
            Arg::Cond(cond, ty, val) => write!(f, "{cond} {ty} {val}"),
            Arg::Phi(ty, values) => {
                write!(f, "{ty}")?;
                let mut first = true;
                for (val, label) in values {
                    if !first {
                        f.write_str(",")?;
                    }
                    first = false;
                    write!(f, " [{val}, {label}]")?;
                }
                Ok(())
            }
            Arg::Call(ty, name, args) => {
                write!(f, "{ty} {name}(")?;
                let mut first = true;
                for arg in args {
                    if !first {
                        f.write_str(", ")?;
                    }
                    first = false;
                    arg.fmt(f)?;
                }
                f.write_str(")")
            }
        }
    }
}

impl fmt::Display for GlobalName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

impl fmt::Display for LocalName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Lit(lit) => lit.fmt(f),
            Val::Local(name) => name.fmt(f),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::I16 => f.write_str("i16"),
            Type::Ptr => f.write_str("ptr"),
            Type::Struct(fields) => {
                f.write_str("{")?;
                if let [first, rest @ ..] = fields.as_slice() {
                    write!(f, "{}", first)?;
                    for field in rest {
                        write!(f, ", {field}")?;
                    }
                }
                f.write_str("}")
            }
            Type::Array(n, elem) => write!(f, "[{n} x {elem}]"),
            Type::Bool => f.write_str("i1"),
        }
    }
}

impl fmt::Display for Lit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lit::I16(n) => write!(f, "{n}"),
            Lit::Null => f.write_str("null"),
            Lit::Struct(fields) => {
                f.write_str("{")?;
                if let [(first_ty, first_lit), rest @ ..] = fields.as_slice() {
                    write!(f, "{first_ty} {first_lit}")?;
                    for (ty, field) in rest {
                        write!(f, ", {ty} {field}")?;
                    }
                }
                f.write_str("}")
            }
            Lit::Array(elems) => {
                f.write_str("[")?;
                if let [(first_ty, first_lit), rest @ ..] = elems.as_slice() {
                    write!(f, "{first_ty} {first_lit}")?;
                    for (ty, lit) in rest {
                        write!(f, ", {ty} {lit}")?;
                    }
                }
                f.write_str("]")
            }
            Lit::Bool(b) => write!(f, "{}", *b as u8),
        }
    }
}

impl fmt::Display for Cond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Cond::Eq => "eq",
            Cond::Ne => "ne",
            Cond::Ugt => "ugt",
            Cond::Uge => "uge",
            Cond::Ult => "ult",
            Cond::Ule => "ule",
            Cond::Sgt => "sgt",
            Cond::Sge => "sge",
            Cond::Slt => "slt",
            Cond::Sle => "sle",
        })
    }
}
