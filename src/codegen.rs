use std::{fmt::Write as _, fs::File, io::Write as _, mem};

use arrayvec::ArrayVec;
use rustc_hash::FxHashMap;

use crate::{lexer::BinOp, parser::{Arg, ExprAst, StatementAst, TopLevelAst}, types::{FunctionType, Type}};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Reg {
    Rax,
    Rbx,
    Rcx,
    Rdx,

    Rbp, // stack frame start
    Rsp, // return pointer
    Rsi,
    Rdi,
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

// TODO : maybe refactor the AsmType passed everywhere, to have the infos in the reg (need for it to become a struct)
impl Reg {
    fn to_addressing_str(self, reg_type : AsmType) -> &'static str {
        match self {
            Reg::Rax => match reg_type {
                AsmType::Qword => "rax",
                AsmType::Dword => "eax",
                AsmType::Word => "ax",
                AsmType::Byte => "al",
            },
            Reg::Rbx => match reg_type {
                AsmType::Qword => "rbx",
                AsmType::Dword => "ebx",
                AsmType::Word => "bx",
                AsmType::Byte => "bl",
            },
            Reg::Rcx => match reg_type {
                AsmType::Qword => "rcx",
                AsmType::Dword => "ecx",
                AsmType::Word => "cx",
                AsmType::Byte => "cl",
            },
            Reg::Rdx => match reg_type {
                AsmType::Qword => "rdx",
                AsmType::Dword => "edx",
                AsmType::Word => "dx",
                AsmType::Byte => "dl",
            },
            Reg::Rsi => match reg_type {
                AsmType::Qword => "rsi",
                AsmType::Dword => "esi",
                AsmType::Word => "si",
                AsmType::Byte => "sil",
            },
            Reg::Rdi => match reg_type {
                AsmType::Qword => "rdi",
                AsmType::Dword => "edi",
                AsmType::Word => "di",
                AsmType::Byte => "dil",
            }
            Reg::R8 => match reg_type {
                AsmType::Qword => "r8",
                AsmType::Dword => "r8d",
                AsmType::Word => "r8w",
                AsmType::Byte => "r8b",
            },
            Reg::R9 => match reg_type {
                AsmType::Qword => "r9",
                AsmType::Dword => "r9d",
                AsmType::Word => "r9w",
                AsmType::Byte => "r9b",
            },
            Reg::R10 => match reg_type {
                AsmType::Qword => "r10",
                AsmType::Dword => "r10d",
                AsmType::Word => "r10w",
                AsmType::Byte => "r10b",
            }
            Reg::R11 => match reg_type {
                AsmType::Qword => "r11",
                AsmType::Dword => "r11d",
                AsmType::Word => "r11w",
                AsmType::Byte => "r11b",
            }
            Reg::R12 => match reg_type {
                AsmType::Qword => "r12",
                AsmType::Dword => "r12d",
                AsmType::Word => "r12w",
                AsmType::Byte => "r12b",
            }
            Reg::R13 => match reg_type {
                AsmType::Qword => "r13",
                AsmType::Dword => "r13d",
                AsmType::Word => "r13w",
                AsmType::Byte => "r13b",
            }
            Reg::R14 => match reg_type {
                AsmType::Qword => "r14",
                AsmType::Dword => "r14d",
                AsmType::Word => "r14w",
                AsmType::Byte => "r14b"
            }
            Reg::R15 => match reg_type {
                AsmType::Qword => "r15",
                AsmType::Dword => "r15d",
                AsmType::Word => "r15w",
                AsmType::Byte => "r15b",
            }
            Reg::Rbp => match reg_type {
                AsmType::Qword => "rbp",
                AsmType::Dword => "ebp",
                AsmType::Word => "bp",
                AsmType::Byte => "bpl",
            }
            Reg::Rsp => match reg_type {
                AsmType::Qword => "rsp",
                AsmType::Dword => "esp",
                AsmType::Word => "sp",
                AsmType::Byte => "spl",
            }
            Reg::Count => unreachable!(),
        }
    }
}

const ASM_TEMPLATE : &str = "
    .intel_syntax noprefix
    .text
";

pub(crate) struct Var {
    pub var_type: Type,
    stack_offset : Option<u32>,
}

struct CodegenContext {
    asm_out : String,
    value_buf : String,
    used_regs : [bool; Reg::Count as usize], // TODO : better reg allocation
    vars : FxHashMap<String, Var>,
    current_stack_offset : u32,
    current_fun_return_type : Type,
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
            current_fun_return_type: Type::Int, // unused value
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
    fn reset_stack_offset(&mut self){
        self.current_stack_offset = 0;
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
                write!(s, "[{}{}{}]", reg.to_addressing_str(AsmType::Qword), sign, off.abs()).unwrap();
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
    fn to_addressing_str(self, s : &mut String, reg_type : AsmType){
        s.clear();
        match self {
            WriteVal::Reg(reg) => s.push_str(reg.to_addressing_str(reg_type)),
            WriteVal::Mem(mem_addr) => mem_addr.to_addressing_str(s),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Value {
    Reg(Reg),
    Mem(MemAddr),
    Constant(i32),
}

impl Value {
    fn to_addressing_str(self, s : &mut String, reg_type : AsmType){
        s.clear();
        match self {
            Value::Constant(nb) => write!(s, "{}", nb).unwrap(),
            Value::Reg(reg) => WriteVal::Reg(reg).to_addressing_str(s, reg_type),
            Value::Mem(mem) => WriteVal::Mem(mem).to_addressing_str(s, reg_type),
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
                emit_mov(codegen_context, write_val, self, AsmType::Qword); // TODO : change the movtype ?
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

fn emit_binary_instr(codegen_context : &mut CodegenContext, instruction : &'static str, to : WriteVal, from : Value, reg_type : AsmType){
    to.to_addressing_str(&mut codegen_context.value_buf, reg_type);
    write!(&mut codegen_context.asm_out, "\t{} {}, ", instruction, codegen_context.value_buf).unwrap();
    from.to_addressing_str(&mut codegen_context.value_buf, reg_type);
    writeln!(&mut codegen_context.asm_out, "{}", codegen_context.value_buf).unwrap();
}

#[derive(Debug, Clone, Copy)]
enum AsmType {
    Byte,
    Word,
    Dword,
    Qword,
}

fn emit_mov(codegen_context : &mut CodegenContext, to : WriteVal, from : Value, reg_type : AsmType){
    // no need for mov from one reg to the same reg
    if let Some(write_val) = from.as_write_val() && write_val == to {
        return;
    }
    
    let mut instruction = "mov";
    // TODO : improve this
    if matches!(to, WriteVal::Mem(_)){
        instruction = match reg_type {
            AsmType::Dword => {
                "mov dword ptr"
            },
            _ => todo!(),
        };
    }
    emit_binary_instr(codegen_context, instruction, to, from, reg_type);
}


fn codegen_number(nb : u128) -> Value {
    let nb : i32 = nb.try_into().unwrap();
    Value::Constant(nb)
}

// TODO : need to add the type conversions (for ex when adding a constant that has been put in a 64 bit reg and a 32 bit add with a var)

fn codegen_binop(codegen_context : &mut CodegenContext, lhs : &ExprAst, op : BinOp, rhs : &ExprAst, expr_type : &Type) -> Value {
    let mut lhs_val = codegen_expr(codegen_context, lhs);
    let mut rhs_val = codegen_expr(codegen_context, rhs);
    match (lhs_val, rhs_val){
        // TODO : add also mem
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

    let reg_type = asm_type_from_type(expr_type);
    emit_binary_instr(codegen_context, instruction, res_write_val, rhs_val, reg_type);
    
    codegen_context.unused_value(rhs_val);

    res_write_val.into()
}

fn codegen_var_use(codegen_context : &mut CodegenContext, var_name : &str) -> Value {
    let reg = codegen_context.next_reg().unwrap();
    let var = codegen_context.vars.get(var_name).unwrap();
    let stack_offset = var.stack_offset.unwrap();
    let var_type = &var.var_type;
    let mov_type = asm_type_from_type(var_type);
    emit_mov(codegen_context, WriteVal::Reg(reg), Value::Mem(MemAddr::Offset { reg: Reg::Rbp, off: -(stack_offset as i32) }), mov_type);
    Value::Reg(reg)
}

// TODO : save register that need to be saved when called (and restored after), also save registers at the prologue, epilogue of function that need to be saved https://s-mazigh.github.io/ASMx86_64/x86_64-LesBases.html

fn codegen_function_call(codegen_context : &mut CodegenContext, fun : &ExprAst, args : &[ExprAst]) -> Value {
    let ret_type = fun.get_type(&codegen_context.vars).into_function_type().unwrap().ret_type;
    let asm_ret_type = asm_type_from_type(&ret_type);
    let mut args_values = args.iter().map(|arg| codegen_expr(codegen_context, arg)).collect::<Vec<_>>();
    let args_asm_types = args.iter().map(|arg| arg.get_type(&codegen_context.vars)).map(|arg_type| asm_type_from_type(&arg_type)).collect::<Vec<_>>();

    // TODO : make used the args regs ? to not have to move them then ?
    let regs_used = ARG_REGS.iter().take(args.len()).copied();
    for (reg_idx, reg) in regs_used.enumerate() {
        if let Some(pos) = args_values.iter().position(|e| e == &Value::Reg(reg)){
            if reg == ARG_REGS[reg_idx] {
                continue;
            }
            let replacement_reg = codegen_context.next_reg().unwrap();
            emit_mov(codegen_context, WriteVal::Reg(replacement_reg), Value::Reg(reg), args_asm_types[pos]);
            args_values[pos] = Value::Reg(replacement_reg);
            codegen_context.unused_value(Value::Reg(reg));
        }
    }

    if let Some(pos) = args_values.iter().position(|e| e == &Value::Reg(Reg::Rax)){
        let replacement_reg = codegen_context.next_reg().unwrap();
        emit_mov(codegen_context, WriteVal::Reg(replacement_reg), Value::Reg(Reg::Rax), asm_ret_type);
        args_values[pos] = Value::Reg(replacement_reg);
    }

    for (arg_idx, &arg) in args_values.iter().enumerate() {
        emit_mov(codegen_context, WriteVal::Reg(ARG_REGS[arg_idx]), arg, args_asm_types[arg_idx]);
    }

    match fun {
        ExprAst::VarUse(fun_name) => {
            writeln!(codegen_context.asm_out, "\tcall {}", fun_name).unwrap();
        }
        _ => todo!(), // TODO : indirect call (need to put the function pointer in a reg)
    }
    codegen_context.used_regs[Reg::Rax as usize] = true;
    for arg in args_values {
        codegen_context.unused_value(arg);
    }

    Value::Reg(Reg::Rax)
}

fn codegen_expr(codegen_context : &mut CodegenContext, ast : &ExprAst) -> Value {
    match ast {
        ExprAst::Number(nb) => codegen_number(*nb),
        ExprAst::VarUse(var_name) => codegen_var_use(codegen_context, var_name),
        ExprAst::BinOp { lhs, op, rhs } => {
            let expr_type = ast.get_type(&codegen_context.vars);
            codegen_binop(codegen_context, lhs.as_ref(), *op, rhs.as_ref(), &expr_type)
        },
        ExprAst::FunctionCall { fun, args } => {
            codegen_function_call(codegen_context, fun.as_ref(), args)
        }
        //_ => panic!("Unknown ast node : {:?}", ast),
    }
}

fn codegen_return(codegen_context : &mut CodegenContext, val : &ExprAst) {
    let val = codegen_expr(codegen_context, val);
    let mov_type = asm_type_from_type(&codegen_context.current_fun_return_type);
    emit_mov(codegen_context, WriteVal::Reg(Reg::Rax), val, mov_type);
    codegen_context.asm_out.push_str(FUN_EPILOGUE);
    codegen_context.asm_out.push_str("\tret\n"); // TODO : have a method on a new type for the tab
}

fn type_size(var_type : &Type) -> u32 {
    match var_type {
        Type::Char => 1,
        Type::Short => 2,
        Type::Int => 4,
        Type::Long | Type::Function(_) => 8,
    }
}

fn asm_type_from_type(t : &Type) -> AsmType {
    match t {
        Type::Char => AsmType::Byte,
        Type::Short => AsmType::Word,
        Type::Int => AsmType::Dword,
        Type::Long | Type::Function(_) => AsmType::Qword,
    }
}

fn codegen_var_decl(codegen_context : &mut CodegenContext, name : &str, var_type : &Type, val : &ExprAst){
    // TODO : add real scopes support (to reuse stack vars)

    let val = codegen_expr(codegen_context, val);
    let type_size = type_size(var_type);
    let stack_offset = codegen_context.get_var_stack_offset(type_size);
    codegen_context.vars.insert(name.to_string(), Var { 
        var_type: var_type.clone(), 
        stack_offset: Some(stack_offset),
    });
    let var_mem = WriteVal::Mem(MemAddr::Offset { reg: Reg::Rbp, off: -(stack_offset as i32) });
    let mov_type = asm_type_from_type(var_type);
    emit_mov(codegen_context, var_mem, val, mov_type);
    codegen_context.unused_value(val);
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

// TODO : omit the function prelude and epilogue if functions don't call any other functions and use no vars

const FUN_PRELUDE : &str = "    push rbp
    mov rbp, rsp
";

const FUN_EPILOGUE : &str = "\tpop rbp\n";

const ARG_REGS: &[Reg] = &[
    Reg::Rdi,
    Reg::Rsi,
    Reg::Rdx,
    Reg::Rcx,
    Reg::R8,
    Reg::R9,
];

const MAX_ARGS : usize = ARG_REGS.len();

// TODO : add .type	[FUNCTION NAME], @function and add .size [FUNCTION NAME], .-[FUNCTION NAME] like in gcc
fn codegen_function(codegen_context : &mut CodegenContext, name : &str, body: &[StatementAst], return_type : &Type, args : &[Arg]){
    if args.len() > MAX_ARGS {
        panic!("too much args"); // TODO : remove maximum (pass on the stack ? see amd64 c abi)
    }
    codegen_context.current_fun_return_type = return_type.clone();
    writeln!(&mut codegen_context.asm_out, "\t.globl	{}", name).unwrap();
    writeln!(&mut codegen_context.asm_out, "{}:", name).unwrap();
    codegen_context.asm_out.push_str(FUN_PRELUDE);
    codegen_context.used_regs = init_used_regs();

    // TODO : replace this vec with a smallvec ? (after removing maximum of args)
    let mut stack_offsets = ArrayVec::<u32, 6>::new();
    for arg in args {
        let var_size = type_size(&arg.arg_type);
        let stack_offset = codegen_context.get_var_stack_offset(var_size);
        stack_offsets.push(stack_offset);
        codegen_context.vars.insert(arg.name.clone(), Var { 
            var_type: arg.arg_type.clone(), 
            stack_offset: Some(stack_offset),
        });
    }

    for arg_idx in 0..args.len(){
        let reg_arg = ARG_REGS[arg_idx];
        let reg_stack_offset = stack_offsets[arg_idx];
        let mem_addr = MemAddr::Offset { reg: Reg::Rbp, off: -(reg_stack_offset as i32) };
        let arg_type = &args[arg_idx].arg_type;
        let mov_type = asm_type_from_type(arg_type);
        emit_mov(codegen_context, WriteVal::Mem(mem_addr), Value::Reg(reg_arg), mov_type);
    }

    for statement in body {
        codegen_statement(codegen_context, statement);
    }

    for arg in args {
        codegen_context.vars.remove(&arg.name);
    }
    codegen_context.reset_stack_offset();

    let args_type = args.iter().map(|arg| arg.arg_type.clone()).collect::<Box<[_]>>();
    codegen_context.vars.insert(name.to_string(), Var { var_type: Type::Function(Box::new(FunctionType {
        ret_type: return_type.clone(),
        args_type,
    })), stack_offset: None });
}

fn codegen_toplevel(codegen_context : &mut CodegenContext, top_level_ast : &TopLevelAst){
    match top_level_ast {
        TopLevelAst::Function { name, body, return_type, args } => codegen_function(codegen_context, name, body, return_type, args),
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