//! Mini-Alive syntax.

pub mod ast;
mod build;
pub mod error;
pub mod inst;
pub mod lex;
pub mod parse;
mod scan;
pub mod source;
#[cfg(test)]
mod tests;
