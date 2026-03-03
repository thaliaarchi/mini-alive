//! S-expression pretty-printing.
//!
//! It uses a simple greedy algorithm, which should give decent results without
//! a general document algebra.

use std::iter;

use crate::smt::smtlib::SExp;

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
