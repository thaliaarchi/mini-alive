use std::{env, fs, process::exit};

use mini_alive::syntax::lex::{Lexer, Token};

fn main() {
    let mut args = env::args_os();
    if args.len() != 2 {
        eprintln!("Usage: mini-alive file.ll");
        exit(2)
    }
    let filename = args.nth(1).unwrap();
    let src = match fs::read_to_string(&filename) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("{}: {err}", filename.display());
            exit(1);
        }
    };
    let mut lex = Lexer::new(&src);
    while let Some(tok) = lex.next() {
        if tok == Token::Invalid {
            eprintln!("Error: Invalid token {:?} at {}", lex.text(), lex.span());
        } else {
            println!("{tok:?} {:?}", lex.text());
        }
    }
}
