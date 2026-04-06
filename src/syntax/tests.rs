use crate::syntax::{
    ast::{Lit, Type},
    parse::Parser,
    source::SourceFile,
};

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
        let ty_src = SourceFile::new(ty_str.into(), "test".into());
        assert_eq!(Parser::new(&ty_src).parse_type(), Ok(ty));
        let lit_src = SourceFile::new(lit_str.into(), "test".into());
        assert_eq!(Parser::new(&lit_src).parse_lit(), Ok(lit));
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

#[test]
fn bools() {
    assert_eq!(Type::Bool.to_string(), "i1");
    let src = SourceFile::new("i1".into(), "test".into());
    assert_eq!(Parser::new(&src).parse_type(), Ok(Type::Bool));
    let tests = [(Lit::Bool(false), "0"), (Lit::Bool(true), "1")];
    for (lit, lit_str) in tests {
        assert_eq!(lit.to_string(), lit_str, "{lit:?}.to_string()");
        assert_eq!(lit.ty(), Some(Type::Bool));
        assert!(lit.has_type(&Type::Bool));
        assert!(lit.valid());
    }
}

#[test]
fn instructions() {
    let src = "\
ret i16 5
ret { i16, i16 } { i16 4, i16 2 }
ret {[3 x i16], {ptr, {}}}
    {[3 x i16] [i16 1, i16 2, i16 3], {ptr, {}} {ptr null, {} {}}}
";
    let src = SourceFile::new(src.into(), "test".into());
    let mut parser = Parser::new(&src);
    let mut insts = Vec::new();
    while !parser.eof() {
        insts.push(parser.parse_inst().unwrap().to_string());
    }
    assert_eq!(
        insts,
        [
            "ret i16 5",
            "ret {i16, i16} {i16 4, i16 2}",
            "ret {[3 x i16], {ptr, {}}} {[3 x i16] [i16 1, i16 2, i16 3], {ptr, {}} {ptr null, {} {}}}"
        ]
    );
}

#[test]
fn module() {
    let tests = [
        "\
define {[3 x i16], {ptr, {}}} @src() {
  ret {[3 x i16], {ptr, {}}} {[3 x i16] [i16 1, i16 2, i16 3], {ptr, {}} {ptr null, {} {}}}
}

declare {[3 x i16], {ptr, {}}} @src2()
",
        "\
define i16 @popcnt(i16 %x) {
entry:
  br label %while.cond

while.cond:
  %x.addr.0 = phi i16 [ %x, %entry ], [ %and, %while.body ]
  %c.0 = phi i16 [ 0, %entry ], [ %inc, %while.body ]
  %tobool.not = icmp eq i16 %x.addr.0, 0
  br i1 %tobool.not, label %while.end, label %while.body

while.body:
  %sub = add i16 %x.addr.0, -1
  %and = and i16 %x.addr.0, %sub
  %inc = add i16 %c.0, 1
  br label %while.cond

while.end:
  ret i16 %c.0
}
",
        "declare i16 @popcnt(i16 %x)\n",
    ];
    for src in tests {
        let src = SourceFile::new(src.into(), "test".into());
        let mut parser = Parser::new(&src);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.to_string(), src.text());
        assert!(parser.eof());
    }
}

#[test]
fn diagnostics() {
    let tests = [
        (
            "define i16 src() {",
            "\
Error: expected global name (@); found identifier `src`
 --> errs.ll:1:12-1:15
  |
1 | define i16 src() {
  |            ^^^
  |
  = context: parsing a function
",
        ),
        (
            "
define {[0 x i16], ptr} @src() { ret label l2 }",
            "\
Error: unknown type name; found identifier `label`
 --> errs.ll:2:38-2:43
  |
2 | define {[0 x i16], ptr} @src() { ret label l2 }
  |                                      ^^^^^
  |
  = context: parsing a type
",
        ),
        (
            "

define i16 @src() {
  %x = extractvalue {[3 x i16], {ptr, {}}} {[3 x i16] [i16 1, i16 2, 3], {ptr, {}} {ptr null, {} {}}}, 0, 1
  ret i16 %x
}
","\
Error: expected identifier, `{`, or `[`; found integer literal `3`
 --> errs.ll:4:70
  |
4 |   %x = extractvalue {[3 x i16], {ptr, {}}} {[3 x i16] [i16 1, i16 2, 3], {ptr, {}} {ptr null, {} {}}}, 0, 1
  |                                                                      ^
  |
  = context: parsing a type
"
        )
    ];
    for (src, diagnostic) in tests {
        let src = SourceFile::new(src.into(), "errs.ll".into());
        let mut parser = Parser::new(&src);
        let err = parser.parse_module().unwrap_err();
        assert_eq!(err.to_string(), diagnostic);
    }
}
