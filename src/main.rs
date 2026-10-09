use std::{env, fs};

use crate::{assembler::assemble, codegen::codegen, lexer::lex, linker::link, parser::parse, preprocessor::preprocess};

mod preprocessor;
mod lexer;
mod parser;
mod types;
mod codegen;
mod assembler;
mod linker;

// TODO : use a arena allocator

// TODO : improve cmd parsing (with a struct, and instead of 2 bool for link and assembling, having an enum to have the target of the tool : linking, assembling, etc)

// TODO : add pointers
// TODO : add sizeof
// TODO : add casts
// TODO : add arrays
// TODO : add bitwise operators
// TODO : add logical operators
// TODO : add continue and break
// TODO : add switch
// TODO : add do while
// TODO : add goto and labels
// TODO : add structs
// TODO : add comma operators (will it create problems when parsing function args ?)

fn main() {
    let mut should_link = true;
    let mut should_assemble = true;
    let mut f = None;
    for arg in env::args(){
        if arg == "-S" {
            should_assemble = false;
            should_link = false;
        } else if arg == "-c" {
            should_link = false;
        } else {
            f = Some(arg);
        }
    }
    let f = f.expect("missing arg");
    let contents = fs::read_to_string(f).unwrap();
    let preprocessed_contents = preprocess(contents);
    let tokens = lex(&preprocessed_contents);
    dbg!(&tokens);
    let ast = parse(tokens);
    dbg!(&ast);
    let out_path = "out.s".as_ref();
    codegen(ast, out_path);
    if should_link {
        link(out_path);
    } else if should_assemble {
        assemble(out_path);
    }
}
