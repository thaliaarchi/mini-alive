//! Mini-Alive syntax.

pub mod inst;
pub mod lex;
pub mod parse;
#[expect(dead_code)]
mod scan;
#[cfg(test)]
mod tests;
pub mod value;
