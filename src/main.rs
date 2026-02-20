use std::{env, fs, process::exit};

use mini_alive::syntax::parse::Parser;

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
    let mut parser = Parser::new(&src);
    println!("Parsed functions:");
    while !parser.eof() {
        match parser.parse_func() {
            Ok(func) => {
                println!("; Debug: {func:?}\n");
                println!("{func}");
            }
            Err(err) => {
                eprint!("{err}");
                break;
            }
        }
    }
}
