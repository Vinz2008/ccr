use std::{fmt::Write as _, fs::File, io::Write as _, mem};

use rustc_hash::FxHashMap;

use crate::{lexer::{BinOp, Type}, parser::{ExprAst, StatementAst, TopLevelAst}};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Reg {
    Rax,
    Rbx,
    Rcx,
    Rdx,

    Rbp, // stack frame start
    Rsp, // return pointer
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
            Reg::Rbp => "rbp",
            Reg::Rsp => "rsp",
            Reg::Count => unreachable!(),
        }
    }
}

const ASM_TEMPLATE : &str = "
    .intel_syntax noprefix
    .text
";

struct Var {
    var_type: Type,
    stack_offset : u32,
}

struct CodegenContext {
    asm_out : String,
    value_buf : String,
    used_regs : [bool; Reg::Count as usize], // TODO : better reg allocation
    vars : FxHashMap<String, Var>,
    current_stack_offset : u32,
}

const fn init_used_regs() -> [bool; Reg::Count as usize] {
    let mut used_regs = [false; Reg::Count as usize];
    used_regs[Reg::Rbp as usize] = true;
    used_regs[Reg::Rsp as usize] = true;
    used_regs
}

impl CodegenContext {
    fn new() -> CodegenContext {
        CodegenContext { 
            asm_out: String::from(ASM_TEMPLATE),
            value_buf : String::with_capacity(3),
            used_regs: init_used_regs(),
            vars: FxHashMap::default(),
            current_stack_offset : 0,
        }
    }

    fn next_reg(&mut self) -> Option<Reg> {
        for i in 0..(Reg::Count as u8) {
            if !self.used_regs[i as usize] {
                self.used_regs[i as usize] = true;
                let reg = unsafe {
                    std::mem::transmute::<u8, Reg>(i)
                };
                return Some(reg);
            }
        }
        None
    }

    fn unused_value(&mut self, val : Value){
        let reg = match val {
            Value::Reg(reg) => reg,
            _ => return,
        };
        let reg_idx = reg as usize;
        self.used_regs[reg_idx] = false;
        
    }

    fn get_var_stack_offset(&mut self, size : u32) -> u32 {
        self.current_stack_offset += size;
        self.current_stack_offset
    }
}

// TODO : use a struct instead (a lot of combinations, see https://blog.yossarian.net/2020/06/13/How-x86_64-addresses-memory)
#[derive(Debug, Clone, Copy, PartialEq)]
enum MemAddr {
    Offset {
        reg : Reg,
        off : i32,
    }
}

impl MemAddr {
    fn to_addressing_str(self, s : &mut String){
        match self {
            MemAddr::Offset { reg, off } => {
                let sign = if off < 0 {
                    '-'
                } else {
                    '+'
                };
                write!(s, "[{}{}{}]", reg.to_addressing_str(), sign, off.abs()).unwrap();
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum WriteVal {
    Reg(Reg),
    Mem(MemAddr),
}

impl WriteVal {
    fn to_addressing_str(self, s : &mut String){
        s.clear();
        match self {
            WriteVal::Reg(reg) => s.push_str(reg.to_addressing_str()),
            WriteVal::Mem(mem_addr) => mem_addr.to_addressing_str(s),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Value {
    Reg(Reg),
    Mem(MemAddr),
    Constant(i32),
}

impl Value {
    fn to_addressing_str(self, s : &mut String){
        s.clear();
        match self {
            Value::Constant(nb) => write!(s, "{}", nb).unwrap(),
            Value::Reg(reg) => WriteVal::Reg(reg).to_addressing_str(s),
            Value::Mem(mem) => WriteVal::Mem(mem).to_addressing_str(s),
        }
    }

    fn from_write_val(write_val : WriteVal) -> Value {
        match write_val {
            WriteVal::Reg(reg) => Value::Reg(reg),
            WriteVal::Mem(mem_addr) => Value::Mem(mem_addr),
        }
    }

    fn into_write_val(self, codegen_context : &mut CodegenContext) -> WriteVal {
        match self {
            Value::Reg(reg) => WriteVal::Reg(reg),
            Value::Mem(mem) => WriteVal::Mem(mem),
            Value::Constant(_) => {
                // TODO : spill to memory if no reg left
                let reg = codegen_context.next_reg().unwrap();
                let write_val = WriteVal::Reg(reg);
                emit_mov(codegen_context, write_val, self, MovType::Qword); // TODO : change the movtype ?
                write_val
            }
        }
    }

    fn as_write_val(self) -> Option<WriteVal> {
        let write_val = match self {
            Value::Reg(reg) => WriteVal::Reg(reg),
            Value::Mem(mem) => WriteVal::Mem(mem),
            Value::Constant(_) => return None,
        };
        Some(write_val)
    }
}

impl From<WriteVal> for Value {
    fn from(value: WriteVal) -> Self {
        Value::from_write_val(value)
    }
}

fn emit_binary_instr(codegen_context : &mut CodegenContext, instruction : &'static str, to : WriteVal, from : Value){
    to.to_addressing_str(&mut codegen_context.value_buf);
    write!(&mut codegen_context.asm_out, "\t{} {}, ", instruction, codegen_context.value_buf).unwrap();
    from.to_addressing_str(&mut codegen_context.value_buf);
    writeln!(&mut codegen_context.asm_out, "{}", codegen_context.value_buf).unwrap();
}

enum MovType {
    Byte,
    Word,
    Dword,
    Qword,
}

fn emit_mov(codegen_context : &mut CodegenContext, to : WriteVal, from : Value, mov_type : MovType){
    // no need for mov from one reg to the same reg
    if let Some(write_val) = from.as_write_val() && write_val == to {
        return;
    }
    let mut instruction = "mov";
    // TODO : improve this
    if matches!(to, WriteVal::Mem(_)){
        instruction = match mov_type {
            MovType::Dword => "mov dword ptr",
            _ => todo!(),
        };
    }
    emit_binary_instr(codegen_context, instruction, to, from);
}


fn codegen_number(nb : u128) -> Value {
    let nb : i32 = nb.try_into().unwrap();
    Value::Constant(nb)
}

// TODO : register allocation

fn codegen_binop(codegen_context : &mut CodegenContext, lhs : &ExprAst, op : BinOp, rhs : &ExprAst) -> Value {
    let mut lhs_val = codegen_expr(codegen_context, lhs);
    let mut rhs_val = codegen_expr(codegen_context, rhs);
    match (lhs_val, rhs_val){
        (Value::Constant(_), Value::Reg(_)) => {
            mem::swap(&mut lhs_val, &mut rhs_val);
        }
        _ => {}
    }

    let res_write_val = lhs_val.into_write_val(codegen_context);
    let instruction = match op {
        BinOp::Plus => "add",
        BinOp::Minus => "sub",
        BinOp::Mult => "imul",
        BinOp::Div => "idiv",
    };
    emit_binary_instr(codegen_context, instruction, res_write_val, rhs_val);
    
    codegen_context.unused_value(rhs_val);

    res_write_val.into()
}

fn codegen_expr(codegen_context : &mut CodegenContext, ast : &ExprAst) -> Value {
    match ast {
        ExprAst::Number(nb) => codegen_number(*nb),
        ExprAst::BinOp { lhs, op, rhs } => codegen_binop(codegen_context, lhs.as_ref(), *op, rhs.as_ref()),
        _ => panic!("Unknown ast node : {:?}", ast),
    }
}

fn codegen_return(codegen_context : &mut CodegenContext, val : &ExprAst) {
    let val = codegen_expr(codegen_context, val);
    let mov_type = MovType::Dword; // TODO : change this
    emit_mov(codegen_context, WriteVal::Reg(Reg::Rax), val, mov_type);
    codegen_context.asm_out.push_str(FUN_EPILOGUE);
    codegen_context.asm_out.push_str("\tret\n"); // TODO : have a method on a new type for the tab
}

fn type_size(var_type : &Type) -> u32 {
    match var_type {
        Type::Int => 4,
    }
}

fn mov_type_from_type(t : &Type) -> MovType {
    match t {
        Type::Int => MovType::Dword,
    }
}

fn codegen_var_decl(codegen_context : &mut CodegenContext, name : &str, var_type : &Type, val : &ExprAst){
    // TODO : add real scopes support (to reuse stack vars)

    let val = codegen_expr(codegen_context, val);
    let type_size = type_size(var_type);
    let stack_offset = codegen_context.get_var_stack_offset(type_size);
    codegen_context.vars.insert(name.to_string(), Var { 
        var_type: var_type.clone(), 
        stack_offset,
    });
    let var_mem = WriteVal::Mem(MemAddr::Offset { reg: Reg::Rbp, off: -(stack_offset as i32) });
    let mov_type = mov_type_from_type(var_type);
    emit_mov(codegen_context, var_mem, val, mov_type);
}

fn codegen_statement(codegen_context : &mut CodegenContext, ast : &StatementAst){
    match ast {
        StatementAst::Return(val) => codegen_return(codegen_context, val.as_ref()),
        StatementAst::Var { name, var_type, val } => codegen_var_decl(codegen_context, name, var_type, val),
        StatementAst::Expr(e) => {
            codegen_expr(codegen_context, e);
        },
    };
}

const FUN_PRELUDE : &str = "    push rbp
    mov rbp, rsp
";

const FUN_EPILOGUE : &str = "\tpop rbp\n";

// TODO : add .type	[FUNCTION NAME], @function and add .size [FUNCTION NAME], .-[FUNCTION NAME] like in gcc
fn codegen_function(codegen_context : &mut CodegenContext, name : &str, body: &[StatementAst], return_type : &Type){
    writeln!(&mut codegen_context.asm_out, "\t.globl	{}", name).unwrap();
    writeln!(&mut codegen_context.asm_out, "{}:", name).unwrap();
    codegen_context.asm_out.push_str(FUN_PRELUDE);
    for statement in body {
        codegen_statement(codegen_context, statement);
    }
}

fn codegen_toplevel(codegen_context : &mut CodegenContext, top_level_ast : &TopLevelAst){
    match top_level_ast {
        TopLevelAst::Function { name, body, return_type } => codegen_function(codegen_context, name, body, return_type),
    }
}

pub(crate) fn codegen(ast : Vec<TopLevelAst>){
    let mut codegen_context = CodegenContext::new();
    for a in ast {
        codegen_toplevel(&mut codegen_context, &a);
    }
    let mut f = File::create("out.s").unwrap();
    f.write_all(codegen_context.asm_out.as_bytes()).unwrap();
}