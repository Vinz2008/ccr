use std::{env, fs};

use crate::{codegen::codegen, lexer::lex, linker::link, parser::parse, preprocessor::preprocess};

mod preprocessor;
mod lexer;
mod parser;
mod types;
mod codegen;
mod linker;

// TODO : use a arena allocator
fn main() {
    let f = env::args().nth(1).expect("missing arg");
    let contents = fs::read_to_string(f).unwrap();
    let preprocessed_contents = preprocess(contents);
    let tokens = lex(&preprocessed_contents);
    dbg!(&tokens);
    let ast = parse(tokens);
    dbg!(&ast);
    codegen(ast);
    link("out.s".as_ref());
}
