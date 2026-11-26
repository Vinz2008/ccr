use std::{fs, process::ExitCode};

use crate::{assembler::assemble, codegen::codegen, lexer::lex, linker::link, options::parse_options, parser::parse, preprocessor::preprocess};

mod options;
mod preprocessor;
mod lexer;
mod parser;
mod types;
mod codegen;
mod assembler;
mod linker;

// TODO : use a arena allocator (https://crates.io/crates/bumpalo or https://crates.io/crates/blink-alloc)

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
    let options = parse_options();
    let contents = fs::read_to_string(options.source_file).unwrap();
    let preprocessed_contents = preprocess(contents);
    let tokens = lex(&preprocessed_contents).unwrap(); // TODO : better error printing (use ariadne)
    println!("[{}:{}:{}] {} = {:#?}", file!(), line!(), column!(), stringify!($val), tokens);
    let ast = parse(tokens).unwrap(); // TODO : better error printing
    // TODO : add optimizations to the ast
    println!("[{}:{}:{}] {} = {:#?}", file!(), line!(), column!(), stringify!($val), ast);
    let asm_path = "out.s".as_ref();
    codegen(ast, asm_path).unwrap(); // TODO : better error printing
    if options.should_link {
        let link_res = link(asm_path, options.output_file.as_ref()); // better error printing
        if let Err(e) = link_res {
            eprintln!("linker failed : {:?}", e); // better error printing (use Display instead ?)
            return ExitCode::FAILURE;
        }
    } else if options.should_assemble {
        assemble(asm_path);
    }
    ExitCode::SUCCESS
}
