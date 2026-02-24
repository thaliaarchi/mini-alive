//! Syntax nodes for values and types.

use std::fmt;

use crate::util::make_enum;

// TODO:
// - Implement type checking: it needs unification for 0-element arrays and
//   boolean literals.

/// A global name (`@`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalName(pub String);

/// A local name (`%`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalName(pub String);

/// A value with an associated type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedVal {
    /// The type of the value.
    pub ty: Type,
    /// A value.
    pub val: Val,
}

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

make_enum! {
    /// Boolean conditional.
    pub enum Cond;
    Eq => "eq",
    Ne => "ne",
    Ugt => "ugt",
    Uge => "uge",
    Ult => "ult",
    Ule => "ule",
    Sgt => "sgt",
    Sge => "sge",
    Slt => "slt",
    Sle => "sle",
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

impl fmt::Display for TypedVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.ty, self.val)
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
