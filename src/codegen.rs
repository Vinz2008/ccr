use std::{fs::File, io::Write as _, fmt::Write as _};

use crate::{lexer::BinOp, parser::AstNode};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Reg {
    Rax,
    Rbx,
    Rcx,
    Rdx,
    // TODO : add rsi, rdi
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    Count,
}

impl Reg {
    fn to_addressing_str(self) -> &'static str {
        match self {
            Reg::Rax => "rax",
            Reg::Rbx => "rbx",
            Reg::Rcx => "rcx",
            Reg::Rdx => "rdx",
            Reg::R8 => "r8",
            Reg::R9 => "r9",
            Reg::R10 => "r10",
            Reg::R11 => "r11",
            Reg::R12 => "r12",
            Reg::R13 => "r13",
            Reg::R14 => "r14",
            Reg::R15 => "r15",
            Reg::Count => unreachable!(),
        }
    }
}

const ASM_TEMPLATE : &'static str = "
    .intel_syntax noprefix
    .text
    .globl \"main\"
main:
";

struct CodegenContext {
    asm_out : String,
    next_reg : Reg,
    value_buf : String,
}

impl CodegenContext {
    fn new() -> CodegenContext {
        CodegenContext { 
            asm_out: String::from(ASM_TEMPLATE),
            next_reg: Reg::Rax,
            value_buf : String::with_capacity(3),
        }
    }

    fn next_reg(&mut self) -> Option<Reg> {
        if self.next_reg == Reg::Count {
            None
        } else {
            let next_reg = self.next_reg;
            self.next_reg = unsafe {
                std::mem::transmute(self.next_reg as u8 + 1)
            };
            Some(next_reg)
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Value {
    Constant(i32),
    Reg(Reg),
}

impl Value {
    fn to_addressing_str(self, s : &mut String){
        s.clear();
        match self {
            Value::Constant(nb) => write!(s, "{}", nb).unwrap(),
            Value::Reg(reg) => s.push_str(reg.to_addressing_str()),
        }
    }

    fn into_reg(self, codegen_context : &mut CodegenContext) -> Reg {
        match self {
            Value::Reg(reg) => reg,
            Value::Constant(_) => {
                let reg = codegen_context.next_reg().unwrap();
                dbg!(reg);
                emit_mov(codegen_context, reg, self);
                reg
            }
        }
    }
}

fn emit_binary_instr(codegen_context : &mut CodegenContext, instruction : &'static str, to : Reg, from : Value){
    from.to_addressing_str(&mut codegen_context.value_buf);
    write!(&mut codegen_context.asm_out, "\t{} {}, {}\n", instruction, to.to_addressing_str(), &codegen_context.value_buf).unwrap();
}

fn emit_mov(codegen_context : &mut CodegenContext, to : Reg, from : Value){
    // no need for mov from one reg to the same reg
    if let Value::Reg(reg) = from && reg == to {
        return;
    }
    emit_binary_instr(codegen_context, "mov", to, from);
}

fn codegen_return(codegen_context : &mut CodegenContext, val : &AstNode) -> Value {
    let val = _codegen(codegen_context, val);
    emit_mov(codegen_context, Reg::Rax, val);
    codegen_context.asm_out.push_str("\tret"); // TODO : have a method on a new type for the tab
    Value::Constant(0) // TODO : separate statements and exprs to not have to return dummy values
}


fn codegen_number(nb : u128) -> Value {
    let nb : i32 = nb.try_into().unwrap();
    Value::Constant(nb)
}

// TODO : register allocation

fn codegen_binop(codegen_context : &mut CodegenContext, lhs : &AstNode, op : BinOp, rhs : &AstNode) -> Value{
    let lhs_val = _codegen(codegen_context, lhs);
    let rhs_val = _codegen(codegen_context, rhs);
    let res_reg = lhs_val.into_reg(codegen_context);
    let instruction = match op {
        BinOp::Plus => "add",
        BinOp::Minus => "sub",
        BinOp::Mult => "imul",
        BinOp::Div => "idiv",
    };
    emit_binary_instr(codegen_context, instruction, res_reg, rhs_val);
    Value::Reg(res_reg)
}

fn _codegen(codegen_context : &mut CodegenContext, ast : &AstNode) -> Value {
    match ast {
        AstNode::Return(val) => codegen_return(codegen_context, val.as_ref()),
        AstNode::Number(nb) => codegen_number(*nb),
        AstNode::BinOp { lhs, op, rhs } => codegen_binop(codegen_context, lhs.as_ref(), *op, rhs.as_ref()),
        _ => panic!("Unknown ast node : {:?}", ast),
    }
}

pub(crate) fn codegen(ast : AstNode){
    let mut codegen_context = CodegenContext::new();
    _codegen(&mut codegen_context, &ast);
    codegen_context.asm_out.push('\n');
    let mut f = File::create("out.s").unwrap();
    f.write_all(codegen_context.asm_out.as_bytes()).unwrap();
}