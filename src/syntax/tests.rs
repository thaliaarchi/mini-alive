use crate::syntax::value::{Lit, Type};

#[test]
fn types_and_literals() {
    let tests = [
        (Type::I16, "i16", Lit::I16(42), "42"),
        (Type::Ptr, "ptr", Lit::Null, "null"),
        (Type::Struct(vec![]), "{}", Lit::Struct(vec![]), "{}"),
        (
            Type::Array(3, Box::new(Type::I16)),
            "[3 x i16]",
            Lit::Array(vec![
                (Type::I16, Lit::I16(1)),
                (Type::I16, Lit::I16(2)),
                (Type::I16, Lit::I16(3)),
            ]),
            "[i16 1, i16 2, i16 3]",
        ),
        (Type::Bool, "i1", Lit::Bool(true), "1"),
        (
            Type::Struct(vec![
                Type::Ptr,
                Type::Array(3, Box::new(Type::I16)),
                Type::Struct(vec![Type::Ptr, Type::Struct(vec![])]),
            ]),
            "{ptr, [3 x i16], {ptr, {}}}",
            Lit::Struct(vec![
                (Type::Ptr, Lit::Null),
                (
                    Type::Array(3, Box::new(Type::I16)),
                    Lit::Array(vec![
                        (Type::I16, Lit::I16(1)),
                        (Type::I16, Lit::I16(2)),
                        (Type::I16, Lit::I16(3)),
                    ]),
                ),
                (
                    Type::Struct(vec![Type::Ptr, Type::Struct(vec![])]),
                    Lit::Struct(vec![
                        (Type::Ptr, Lit::Null),
                        (Type::Struct(vec![]), Lit::Struct(vec![])),
                    ]),
                ),
            ]),
            "{ptr null, [3 x i16] [i16 1, i16 2, i16 3], {ptr, {}} {ptr null, {} {}}}",
        ),
    ];
    for (ty, ty_str, lit, lit_str) in tests {
        assert_eq!(ty.to_string(), ty_str, "{ty:?}.to_string()");
        assert_eq!(lit.to_string(), lit_str, "{lit:?}.to_string()");
        assert_eq!(lit.ty().as_ref(), Some(&ty));
        assert!(lit.has_type(&ty));
    }
}

#[test]
fn empty_array() {
    let ty1 = Type::Array(0, Box::new(Type::I16));
    assert_eq!(ty1.to_string(), "[0 x i16]");
    let ty2 = Type::Array(0, Box::new(Type::Struct(vec![Type::I16, Type::Ptr])));
    assert_eq!(ty2.to_string(), "[0 x {i16, ptr}]");
    let lit = Lit::Array(vec![]);
    assert_eq!(lit.to_string(), "[]");
    assert_eq!(lit.ty(), None);
    assert!(lit.has_type(&ty1));
    assert!(lit.has_type(&ty2));
}
