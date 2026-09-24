use std::{env, fs};

use crate::{lexer::lex, parser::parse, codegen::codegen};

mod lexer;
mod parser;
mod codegen;

// TODO : use a arena allocator
fn main() {
    let f = env::args().nth(1).expect("missing arg");
    let contents = fs::read_to_string(f).unwrap();
    let tokens = lex(&contents);
    dbg!(&tokens);
    let ast = parse(tokens);
    dbg!(&ast);
    codegen(ast);
}
