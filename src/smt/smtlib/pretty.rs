//! S-expression pretty-printing.
//!
//! It uses a simple greedy algorithm, which should give decent results without
//! a general document algebra.

use std::iter;

use crate::smt::smtlib::{SExp, Script};

// TODO:
// - Implement list style for functions, where the name, arguments, and return
//   type are flat and the body is indented by two spaces, e.g.:
//       (define-fun sum32 ((x (_ BitVec 32)) (y (_ BitVec 32))) (_ BitVec 32)
//         (bvadd x y))
// - Unify list style logic. There's some constant number of args at the start
//   that are flat (store as a `u8`). Then, the rest are indented with some
//   strategy: either relative to the start (+0 or +1) or hanging relative to
//   the last flat arg. Can this be represented without growing `SExp`?

/// The style for pretty-printing a list on multiple lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListStyle {
    /// Vertical style, e.g.:
    ///
    /// ```text
    /// (a
    ///  b
    ///  c
    ///  d)
    /// ```
    Vertical,
    /// Argument list style, e.g.:
    ///
    /// ```text
    /// (a b
    ///    c
    ///    d)
    /// ```
    Hanging,
}

impl Script {
    /// Pretty-prints the script.
    pub fn pretty(&self) -> String {
        let mut s = String::new();
        for command in &self.commands {
            s += &command.pretty();
            s.push('\n');
        }
        s
    }
}

impl SExp {
    /// Pretty-prints the S-expression.
    pub fn pretty(&self) -> String {
        PrettyPrinter::new().pretty(self, 0).0
    }
}

/// A pretty-printer for indented S-expressions.
struct PrettyPrinter {
    indent_buf: Vec<u8>,
}

impl PrettyPrinter {
    const INITIAL_INDENT_CAPACITY: usize = 80;
    const MAX_FLAT_LEN: usize = 60;

    /// Constructs a new pretty-printer.
    fn new() -> Self {
        let mut indent_buf = vec![b' '; Self::INITIAL_INDENT_CAPACITY + 1];
        indent_buf[0] = b'\n';
        PrettyPrinter { indent_buf }
    }

    /// Pretty-prints the S-expression at the given indentation level and
    /// returns whether it is multi-line.
    fn pretty(&mut self, sexp: &SExp, indent: usize) -> (String, bool) {
        match sexp {
            SExp::Atom(atom) => {
                let text = atom.to_string();
                (text, false)
            }
            SExp::List(list) => {
                if list.elems.is_empty() {
                    return ("()".to_owned(), false);
                }

                let mut spaces = indent + 1;
                let (first, mut multiline) = self.pretty(&list.elems[0], spaces);
                let style = if multiline {
                    ListStyle::Vertical
                } else {
                    list.style
                };
                if style == ListStyle::Hanging {
                    spaces += first.len() + 1;
                }
                let mut flat_len = first.len() + 2;
                let mut rest = Vec::with_capacity(list.elems.len() - 1);
                for elem in &list.elems[1..] {
                    let (pretty, m) = self.pretty(elem, spaces);
                    flat_len += pretty.len() + 1;
                    rest.push(pretty);
                    multiline |= m;
                }
                if flat_len > Self::MAX_FLAT_LEN {
                    multiline = true;
                }

                let mut s = "(".to_owned();
                s += &first;
                let skip = if style == ListStyle::Hanging && !rest.is_empty() {
                    s.push(' ');
                    s += &rest[0];
                    1
                } else {
                    0
                };
                let indent = if multiline { self.indent(spaces) } else { " " };
                for elem in &rest[skip..] {
                    s += indent;
                    s += elem;
                }
                s += ")";
                (s, multiline)
            }
        }
    }

    /// Returns a string of a given number of spaces, preceded by LF.
    fn indent(&mut self, spaces: usize) -> &str {
        let len = spaces + 1; // Include leading LF
        if let Some(n) = len.checked_sub(self.indent_buf.len()) {
            self.indent_buf.extend(iter::repeat_n(b' ', n));
        }
        unsafe { str::from_utf8_unchecked(&self.indent_buf[..len]) }
    }
}

#[cfg(test)]
mod tests {
    use crate::smt::smtlib::{CommandName, List, ListStyle, Reserved, SExp, Script, Symbol};

    macro_rules! sym(($sym:ident) => {
        SExp::from(Symbol::new(stringify!($sym)))
    });
    macro_rules! list(($style:ident $(: $($elem:expr),+ $(,)?)?) => {
        SExp::from(List {
            elems: vec![$($($elem.into()),+)?],
            style: ListStyle::$style,
        })
    });

    #[test]
    fn lists() {
        assert_eq!(list![Vertical].pretty(), "()");
        assert_eq!(list![Hanging].pretty(), "()");
        assert_eq!(list![Vertical: sym!(x)].pretty(), "(x)");
        assert_eq!(list![Hanging: sym!(x)].pretty(), "(x)");
        assert_eq!(
            list![
                Hanging:
                sym!(bvand),
                sym!(x),
                list![Hanging: Reserved::Underscore, sym!(bv123), 64],
            ]
            .pretty(),
            "(bvand x (_ bv123 64))",
        );
        let long = sym!(the_quick_brown_fox_jumps_over_the_lazy_dog_0123456789);
        assert_eq!(
            list![Vertical: sym!(bvadd), long.clone(), sym!(y23)].pretty(),
            "\
(bvadd
 the_quick_brown_fox_jumps_over_the_lazy_dog_0123456789
 y23)",
        );
        assert_eq!(
            list![Hanging: sym!(bvadd), long.clone(), sym!(y23)].pretty(),
            "\
(bvadd the_quick_brown_fox_jumps_over_the_lazy_dog_0123456789
       y23)",
        );
    }

    #[test]
    fn function_example() {
        // https://microsoft.github.io/z3guide/docs/theories/Bitvectors/

        // (define-fun popcount32 ((v (_ BitVec 32))) (_ BitVec 32)
        //    (let ((v (bvsub v (bvand (bvlshr v (_ bv1 32)) #x55555555))))
        //    (let ((v (bvadd (bvand v #x33333333) (bvand (bvlshr v (_ bv2 32)) #x33333333))))
        //    (bvlshr (bvmul (bvand (bvadd v (bvlshr v (_ bv4 32))) #x0F0F0F0F) #x01010101) (_ bv24 32)))
        //    )
        // )
        //
        // (simplify (popcount32 #x01234100))

        let mut script = Script::new();
        let popcount32 = list![Hanging:
            CommandName::DefineFun,
            sym!(popcount32),
            list![Vertical:
                list![Vertical:
                    sym!(v),
                    list![Hanging: Reserved::Underscore, sym!(BitVec), 32],
                ],
            ],
            list![Hanging: Reserved::Underscore, sym!(BitVec), 32],
            list![Hanging:
                Reserved::Let,
                list![Vertical:
                    list![Vertical:
                        sym!(v),
                        list![Hanging:
                            sym!(bvsub),
                            sym!(v),
                            list![Hanging:
                                sym!(bvand),
                                list![Hanging:
                                    sym!(bvlshr),
                                    sym!(v),
                                    list![Hanging: Reserved::Underscore, sym!(bv1), 32],
                                ],
                                0x55555555,
                            ],
                        ],
                    ],
                ],
                list![Hanging:
                    Reserved::Let,
                    list![Vertical:
                        list![Vertical:
                            sym!(v),
                            list![Hanging:
                                sym!(bvadd),
                                list![Hanging:
                                    sym!(bvand),
                                    sym!(v),
                                    0x33333333,
                                ],
                                list![Hanging:
                                    sym!(bvand),
                                    list![Hanging:
                                        sym!(bvlshr),
                                        sym!(v),
                                        list![Hanging: Reserved::Underscore, sym!(bv2), 32],
                                    ],
                                    0x33333333,
                                ],
                            ],
                        ],
                    ],
                    list![Hanging:
                        sym!(bvlshr),
                        list![Hanging:
                            sym!(bvmul),
                            list![Hanging:
                                sym!(bvand),
                                list![Hanging:
                                    sym!(bvadd),
                                    sym!(v),
                                    list![Hanging:
                                        sym!(bvlshr),
                                        sym!(v),
                                        list![Hanging: Reserved::Underscore, sym!(bv4), 32],
                                    ],
                                ],
                                0x0F0F0F0F,
                            ],
                            0x01010101,
                        ],
                        list![Hanging: Reserved::Underscore, sym!(bv24), 32],
                    ],
                ],
            ],
        ];
        script.push(popcount32);
        script.push(list![Hanging:
            sym!(simplify),
            list![Hanging: sym!(popcount32), 19087616],
        ]);
        assert_eq!(
            script.pretty(),
            "\
(define-fun popcount32
            ((v (_ BitVec 32)))
            (_ BitVec 32)
            (let ((v (bvsub v (bvand (bvlshr v (_ bv1 32)) 1431655765))))
                 (let ((v
                        (bvadd (bvand v 858993459)
                               (bvand (bvlshr v (_ bv2 32)) 858993459))))
                      (bvlshr (bvmul (bvand (bvadd v (bvlshr v (_ bv4 32))) 252645135)
                                     16843009)
                              (_ bv24 32)))))
(simplify (popcount32 19087616))
"
        );
    }
}
