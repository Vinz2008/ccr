use std::cmp;

use rustc_hash::FxHashMap;

use crate::{codegen::Var, lexer::Type, parser::ExprAst};

fn guess_lit_nb_type(nb : u128) -> Type {
    if nb > u64::MAX as u128 {
        panic!("too big of a number");
    }
    if nb > u32::MAX as u128 {
        Type::Long
    } else if nb > u16::MAX as u128 {
        Type::Int
    } else if nb > u8::MAX as u128 {
        Type::Short
    } else {
        Type::Char
    }
}

fn get_binop_type(lhs : &ExprAst, rhs : &ExprAst, vars : &FxHashMap<String, Var>) -> Type {
    let lhs_type = lhs.get_type(vars);
    let rhs_type = rhs.get_type(vars);
    let max_type_size = cmp::max(rhs_type, lhs_type);
    cmp::max(max_type_size, Type::Int)
}

fn get_function_call_type(fun : &ExprAst, vars : &FxHashMap<String, Var>) -> Type {
    fun.get_type(vars).into_function_type().unwrap().ret_type.as_ref().clone()
}

impl ExprAst {
    pub(crate) fn get_type(&self, vars : &FxHashMap<String, Var>) -> Type {
        // TODO : need special handling here for the unary expr - and a static number (if found u32 and then need -, then it becomes a i64 ? check this)
        match self {
            ExprAst::Number(nb) => guess_lit_nb_type(*nb),
            ExprAst::VarUse(ident) => vars.get(ident.as_ref()).unwrap().var_type.clone(),
            ExprAst::BinOp { lhs, op: _, rhs } => get_binop_type(lhs.as_ref(), rhs.as_ref(), vars),
            ExprAst::FunctionCall { fun, args: _ } => get_function_call_type(fun.as_ref(), vars),
        }
    }
}