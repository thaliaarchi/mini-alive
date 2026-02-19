use std::{env, fs, process::exit};

use mini_alive::syntax::{
    lex::{Lexer, Token},
    parse::Parser,
};

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
    let mut lexer = Lexer::new(&src);
    println!("Tokens:");
    loop {
        let lex = lexer.next();
        if lex.tok == Token::Eof {
            break;
        } else if lex.tok == Token::Invalid {
            eprintln!("Error: Invalid token {:?} at {}", lex.text, lex.span);
        } else {
            println!("{:?} {:?}", lex.tok, lex.text);
        }
    }
    println!();

    let mut parser = Parser::new(&src);
    println!("Parsed as type:");
    match parser.parse_type() {
        Ok(ty) => {
            println!("  Debug: {ty:?}");
            println!("  Pretty: {ty}");
        }
        Err(err) => eprintln!("Error: {err:?}"),
    }
    let mut parser = Parser::new(&src);
    println!("Parsed as literal:");
    match parser.parse_lit() {
        Ok(lit) => {
            println!("  Debug: {lit:?}");
            println!("  Pretty: {lit}");
        }
        Err(err) => eprintln!("Error: {err:?}"),
    }
}
