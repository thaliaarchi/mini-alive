//! Mini-Alive syntax nodes.

use std::fmt;

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
    Array(Type, Vec<Lit>),
    /// Boolean: `"0" | "1"`
    Bool(bool),
}

impl Lit {
    /// Gets the type of this literal value.
    pub fn ty(&self) -> Type {
        match self {
            Lit::I16(_) => Type::I16,
            Lit::Null => Type::Ptr,
            Lit::Struct(fields) => Type::Struct(fields.iter().map(|(ty, _)| ty.clone()).collect()),
            Lit::Array(ty, elems) => Type::Array(elems.len(), Box::new(ty.clone())),
            Lit::Bool(_) => Type::Bool,
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
            (Lit::Array(lit_ty, elems), Type::Array(n, ty)) => lit_ty == &**ty && elems.len() == *n,
            (Lit::Bool(_), Type::Bool) => true,
            _ => false,
        }
    }

    /// Checks whether this literal value is valid for its types.
    pub fn valid(&self) -> bool {
        match self {
            Lit::I16(_) | Lit::Null | Lit::Bool(_) => true,
            Lit::Struct(fields) => fields.iter().all(|(ty, field)| field.has_type(ty)),
            Lit::Array(ty, elems) => elems.iter().all(|elem| elem.has_type(ty)),
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
            Lit::Array(ty, elems) => {
                f.write_str("[")?;
                if let [first, rest @ ..] = elems.as_slice() {
                    write!(f, "{ty} {first}")?;
                    for elem in rest {
                        write!(f, ", {ty} {elem}")?;
                    }
                }
                f.write_str("]")
            }
            Lit::Bool(b) => write!(f, "{}", *b as u8),
        }
    }
}
