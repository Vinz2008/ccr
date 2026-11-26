use std::{env, fs, process::{self, ExitCode}};

use crate::{assembler::assemble, codegen::codegen, lexer::lex, linker::link, parser::parse, preprocessor::preprocess};

mod preprocessor;
mod lexer;
mod parser;
mod types;
mod codegen;
mod assembler;
mod linker;

// TODO : use a arena allocator (https://crates.io/crates/bumpalo or https://crates.io/crates/blink-alloc)

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

fn main() -> ExitCode {
    let mut should_link = true;
    let mut should_assemble = true;
    let mut f = None;
    let mut output_file = "a.out".to_string();
    let mut args_iter = env::args().skip(1);
    while let Some(arg) = args_iter.next() {
        if arg == "-S" {
            should_assemble = false;
            should_link = false;
        } else if arg == "-c" {
            should_link = false;
        } else if arg == "-o" {
            output_file = args_iter.next().expect("missing output file after -o");
        } else {
            f = Some(arg);
        }
    }
    let f = f.expect("missing arg");
    let contents = fs::read_to_string(f).unwrap();
    let preprocessed_contents = preprocess(contents);
    let tokens = lex(&preprocessed_contents).unwrap(); // TODO : better error printing (use ariadne)
    println!("[{}:{}:{}] {} = {:#?}", file!(), line!(), column!(), stringify!($val), tokens);
    let ast = parse(tokens).unwrap(); // TODO : better error printing
    // TODO : add optimizations to the ast
    println!("[{}:{}:{}] {} = {:#?}", file!(), line!(), column!(), stringify!($val), ast);
    let asm_path = "out.s".as_ref();
    codegen(ast, asm_path).unwrap(); // TODO : better error printing
    if should_link {
        let link_res = link(asm_path, output_file.as_ref()); // better error printing
        if let Err(e) = link_res {
            eprintln!("linker failed : {:?}", e); // better error printing (use Display instead ?)
            return ExitCode::FAILURE;
        }
    } else if should_assemble {
        assemble(asm_path);
    }
    ExitCode::SUCCESS
}
