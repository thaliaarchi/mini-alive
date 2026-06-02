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
add i16 %0, 1
extractvalue {i16, i16} {i16 4, i16 2}, 0
insertvalue {i16, i16} {i16 4, i16 2}, i16 7, 1
alloca i16, 4
load i16, ptr %p
store i16 %0, ptr %p
icmp eq i16 %1, 0
phi i16 [ %1, %2 ], [ 0, %3 ]
call i16 @f(i16 %1)
ret i16 5
ret { i16, i16 } { i16 4, i16 2 }
ret {[3 x i16], {ptr, {}}}
    {[3 x i16] [i16 1, i16 2, i16 3], {ptr, {}} {ptr null, {} {}}}
br label %done
br i1 %cond, label %t, label %f
";
    let src = SourceFile::new(src.into(), "test".into());
    let mut parser = Parser::new(&src);
    let mut insts = Vec::new();
    while !parser.eof() {
        parser.reset_builder();
        insts.push(parser.parse_inst().unwrap().to_string());
    }
    assert_eq!(
        insts,
        [
            "%0 = add i16 %0, 1",
            "%0 = extractvalue {i16, i16} {i16 4, i16 2}, 0",
            "%0 = insertvalue {i16, i16} {i16 4, i16 2}, i16 7, 1",
            "%0 = alloca i16, 4",
            "%0 = load i16, ptr %p",
            "store i16 %0, ptr %p",
            "%0 = icmp eq i16 %1, 0",
            "%0 = phi i16 [ %1, %2 ], [ 0, %3 ]",
            "%0 = call i16 @f(i16 %1)",
            "ret i16 5",
            "ret {i16, i16} {i16 4, i16 2}",
            "ret {[3 x i16], {ptr, {}}} {[3 x i16] [i16 1, i16 2, i16 3], {ptr, {}} {ptr null, {} {}}}",
            "br label %done",
            "br i1 %cond, label %t, label %f",
        ]
    );
}

#[test]
fn module() {
    let agg = "\
define {[3 x i16], {ptr, {}}} @src() {
0:
  ret {[3 x i16], {ptr, {}}} {[3 x i16] [i16 1, i16 2, i16 3], {ptr, {}} {ptr null, {} {}}}
}

declare {[3 x i16], {ptr, {}}} @src2()
";
    let popcnt = "\
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
";

    let popcnt_numbered = "\
define i16 @popcnt(i16 %0) {
1:
  br label %2

2:
  %3 = phi i16 [ %0, %1 ], [ %8, %6 ]
  %4 = phi i16 [ 0, %1 ], [ %9, %6 ]
  %5 = icmp eq i16 %3, 0
  br i1 %5, label %10, label %6

6:
  %7 = add i16 %3, -1
  %8 = and i16 %3, %7
  %9 = add i16 %4, 1
  br label %2

10:
  ret i16 %4
}
";
    let tests = [
        (agg, agg),
        (popcnt, popcnt),
        (
            "\
define i16 @popcnt(i16) {
  br label %2

  phi i16 [ %0, %1 ], [ %8, %6 ]
  phi i16 [ 0, %1 ], [ %9, %6 ]
  icmp eq i16 %3, 0
  br i1 %5, label %10, label %6

  add i16 %3, -1
  and i16 %3, %7
  add i16 %4, 1
  br label %2

  ret i16 %4
}
",
            popcnt_numbered,
        ),
        (
            "\
define i16@popcnt(i16){br label%2phi i16[%0,%1],[%8,%6]phi i16[0,%1],[%9,%6]icmp
eq i16%3,0br i1%5,label%10,label%6add i16%3,-1and i16%3,%7add i16%4,1br label%2
ret i16%4}
",
            popcnt_numbered,
        ),
        (
            "declare i16 @popcnt(i16 %x)\n",
            "declare i16 @popcnt(i16 %x)\n",
        ),
    ];
    for (src, expected) in tests {
        let src = SourceFile::new(src.into(), "test".into());
        let mut parser = Parser::new(&src);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.to_string(), expected);
        assert!(parser.eof());
    }
}

#[test]
fn normalized_ids() {
    let src = "\
define i16 @00(i16 %00) {
01:
  %3 = icmp eq i16 %0, 0
  br i1 %0003, label %004, label %4
4:
  ret i16 %000
}
";
    let expected = "\
define i16 @0(i16 %0) {
1:
  %3 = icmp eq i16 %0, 0
  br i1 %3, label %4, label %4

4:
  ret i16 %0
}
";
    let src = SourceFile::new(src.into(), "test".into());
    let mut parser = Parser::new(&src);
    let module = parser.parse_module().unwrap();
    assert_eq!(module.to_string(), expected);
}

#[test]
fn unnamed_params() {
    let tests = [
        (
            "declare i16 @f(i16, ptr %p)\n",
            "declare i16 @f(i16 %0, ptr %p)\n",
        ),
        (
            "define i16 @f(i16, ptr %p) {\n  ret i16 0\n}\n",
            "define i16 @f(i16 %0, ptr %p) {\n1:\n  ret i16 0\n}\n",
        ),
    ];
    for (src, expected) in tests {
        let src = SourceFile::new(src.into(), "test".into());
        let mut parser = Parser::new(&src);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.to_string(), expected);
    }
}

#[test]
fn typechecking() {
    let ok_tests = [
        "\
define i16 @f() {
entry:
  %p = alloca i16, 1
  store i16 0, ptr %p
  %x = load i16, ptr %p
  ret i16 %x
}
",
        "\
define i16 @f(i16 %x) {
entry:
  %0 = add i16 %x, 1
  %1 = add i16 %0, %x
  ret i16 %1
}
",
        "\
define ptr @f() {
entry:
  %x = extractvalue {i16, ptr} {i16 1, ptr null}, 1
  ret ptr %x
}
",
        "\
define i16 @f() {
entry:
  %x = extractvalue [2 x i16] [i16 1, i16 2], 1
  ret i16 %x
}
",
        "\
define ptr @f() {
entry:
  %x = extractvalue [2 x {i16, ptr}] [{i16, ptr} {i16 1, ptr null}, {i16, ptr} {i16 2, ptr null}], 1, 1
  ret ptr %x
}
",
    ];
    for src in ok_tests {
        let src = SourceFile::new(src.into(), "test.ll".into());
        let module = Parser::new(&src).parse_module().unwrap();
        assert_eq!(module.to_string(), src.text());
    }

    let err_tests = [
        (
            "\
define i16 @f() {
entry:
  %p = alloca i16, 1
  %x = load i16, i16 %p
  ret i16 %x
}
",
            "\
Error: %p is used with type i16, but expected ptr
 --> errs.ll:4:22-4:24
  |
4 |   %x = load i16, i16 %p
  |                      ^^
  |
",
        ),
        (
            "\
define i16 @f() {
entry:
  br label %loop

loop:
  %x = phi i16 [ 0, %entry ], [ %y, %back ]
  ret i16 %x

back:
  %y = alloca i16, 1
  br label %loop
}
",
            "\
Error: %y is used with type ptr, but expected i16
  --> errs.ll:10:3-10:5
   |
10 |   %y = alloca i16, 1
   |   ^^
   |
",
        ),
        (
            "\
define i16 @src() {
  %x = extractvalue {i16} {i16 1}, 1
  ret i16 %x
}
",
            "\
Error: invalid aggregate index; found integer literal `1`
 --> errs.ll:2:36
  |
2 |   %x = extractvalue {i16} {i16 1}, 1
  |                                    ^
  |
  = context: parsing an `extractvalue` instruction
",
        ),
        (
            "\
define i16 @f() {
entry:
  %x = extractvalue [2 x i16] [i16 1, i16 2], 2
  ret i16 %x
}
",
            "\
Error: invalid aggregate index; found integer literal `2`
 --> errs.ll:3:47
  |
3 |   %x = extractvalue [2 x i16] [i16 1, i16 2], 2
  |                                               ^
  |
  = context: parsing an `extractvalue` instruction
",
        ),
    ];
    for (src, diagnostic) in err_tests {
        let src = SourceFile::new(src.into(), "errs.ll".into());
        let err = Parser::new(&src).parse_module().unwrap_err();
        assert_eq!(err.to_string(), diagnostic);
    }
}

#[test]
fn diagnostics() {
    let tests = [
        (
            "define i16 src() {",
            "\
Error: expected global variable (@); found identifier `src`
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
",
            "\
Error: expected identifier, `{`, or `[`; found integer literal `3`
 --> errs.ll:4:70
  |
4 |   %x = extractvalue {[3 x i16], {ptr, {}}} {[3 x i16] [i16 1, i16 2, 3], {ptr, {}} {ptr null, {} {}}}, 0, 1
  |                                                                      ^
  |
  = context: parsing a type
"
        ),
        (
            "\
define i16 @src() {
  /* unterminated
  ret i16 0
}
",
            "\
Error: expected identifier; found invalid token `/* unterminated\\n  ret i16 0\\n}\\n`
 --> errs.ll:2:3-4:3
  |
2 |   /* unterminated
  |   ^^^^^^^^^^^^^^^
3 |   ret i16 0
  | ^^^^^^^^^^^
4 | }
  | ^^
  |
  = context: parsing the opcode of an instruction
"
        ),
        (
            "\
define i16 @src() {
}
",
            "\
Error: basic block missing terminator; found `}`
 --> errs.ll:2:1
  |
2 | }
  | ^
  |
  = context: parsing a basic block
",
        ),
        (
            "\
define i16 @src() {
  add i16 1, 2
}
",
            "\
Error: basic block missing terminator; found `}`
 --> errs.ll:3:1
  |
3 | }
  | ^
  |
  = context: parsing a basic block
",
        ),
        (
            "\
define i16 @src() {
  br label %1

  add i16 1, 2
}
",
            "\
Error: basic block missing terminator; found `}`
 --> errs.ll:5:1
  |
5 | }
  | ^
  |
  = context: parsing a basic block
",
        ),
        (
            "\
define i16 @f() {
  ret i16 %x
}
",
            "\
Error: undefined value %x
 --> errs.ll:2:11-2:13
  |
2 |   ret i16 %x
  |           ^^
  |
",
        ),
        (
            "\
define i16 @f(i16 %x) {
x:
  ret i16 %x
}
",
            "\
Error: redefined %x
 --> errs.ll:2:1-2:3
  |
2 | x:
  | ^^
  |
",
        ),
        (
            "\
define i16 @f(i16 %2) {
3:
  %1 = add i16 %2, 1
  ret i16 %1
}
",
            "\
Error: %1 is less than the next available ID %4
 --> errs.ll:3:3-3:5
  |
3 |   %1 = add i16 %2, 1
  |   ^^
  |
",
        ),
        (
            "\
define i16 @f() {
entry:
  br label %bb
bb:
  ret i16 %entry
}
",
            "\
Error: %entry is used as a value, but expected a label
 --> errs.ll:5:11-5:17
  |
5 |   ret i16 %entry
  |           ^^^^^^
  |
",
        ),
        (
            "\
define i16 @f(i16 %x) {
entry:
  br label %x
}
",
            "\
Error: %x is used as a label, but expected a value
 --> errs.ll:3:12-3:14
  |
3 |   br label %x
  |            ^^
  |
",
        ),
    ];
    for (src, diagnostic) in tests {
        let src = SourceFile::new(src.into(), "errs.ll".into());
        let err = Parser::new(&src).parse_module().unwrap_err();
        assert_eq!(err.to_string(), diagnostic);
    }
}
