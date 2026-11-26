use rustc_hash::FxHashMap;

use crate::{codegen::{Var, type_size}, parser::{BinaryOp, ExprAst}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FunctionType {
    pub ret_type: Type,
    pub args_type : Box<[Type]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sign {
    Signed,
    Unsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegerType {
    Char,
    Short,
    Int,
    Long,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Type {
    Integer(Sign, IntegerType),
    Function(Box<FunctionType>),
    Ptr(Box<Type>), // TODO : need to add char const*
    Void,
}

impl Type {
    pub(crate) fn into_function_type(self) -> Option<FunctionType> {
        match self {
            Type::Function(func_type) => Some(*func_type),
            _ => None,
        }
    }

    pub(crate) fn as_integer_type(&self) -> Option<(Sign, IntegerType)> {
        match self {
            Type::Integer(sign, i_t) => Some((*sign, *i_t)),
            _ => None,
        }
    }
}

fn guess_lit_nb_type(nb : i128) -> Type {
    // TODO : improve this so I can have u64 literals work (also test lower bounds, like if lower than min because with optimization can have negatives)
    if nb > i64::MAX as i128 {
        panic!("too big of a number");
    }
    if nb > i32::MAX as i128 {
        Type::Integer(Sign::Signed, IntegerType::Long)
    } else {
        Type::Integer(Sign::Signed, IntegerType::Int)
    }
}

fn get_binop_type(lhs : &ExprAst, binop : BinaryOp, rhs : &ExprAst, vars : &FxHashMap<Box<str>, Var>) -> Type {
    match binop {
        BinaryOp::Cmp => Type::Integer(Sign::Signed, IntegerType::Int),
        BinaryOp::Equal => rhs.get_type(vars),
        _ => {
            let lhs_type = lhs.get_type(vars);
            let rhs_type = rhs.get_type(vars);
            // TODO : better type checking
            let lhs_size = type_size(&lhs_type);
            let rhs_size = type_size(&rhs_type);
            let (max_type, max_type_size) = if rhs_size > lhs_size{
                (rhs_type, rhs_size)
            } else {
                (lhs_type, lhs_size)
            };
            let int_type = Type::Integer(Sign::Signed, IntegerType::Int);
            if type_size(&int_type) > max_type_size {
                int_type
            } else {
                max_type
            }
        }
    }
}

fn get_infix_type(val : &ExprAst, vars : &FxHashMap<Box<str>, Var>) -> Type {
    val.get_type(vars)
}

fn get_postfix_type(val : &ExprAst, vars : &FxHashMap<Box<str>, Var>) -> Type {
    val.get_type(vars)
}

fn get_function_call_type(fun : &ExprAst, vars : &FxHashMap<Box<str>, Var>) -> Type {
    fun.get_type(vars).into_function_type().unwrap().ret_type.clone()
}

impl ExprAst {
    pub(crate) fn get_type(&self, vars : &FxHashMap<Box<str>, Var>) -> Type {
        // TODO : need special handling here for the unary expr - and a static number (if found u32 and then need -, then it becomes a i64 ? check this)
        match self {
            ExprAst::Number(nb) => guess_lit_nb_type(*nb),
            ExprAst::Char(_) => Type::Integer(Sign::Signed, IntegerType::Char),
            ExprAst::Sizeof(_) => Type::Integer(Sign::Unsigned, IntegerType::Long),
            ExprAst::VarUse(ident) => vars.get(ident.as_ref()).or_else(|| panic!("var not found : {}", ident)).unwrap().var_type.clone(), // TODO : better error handling (with a TypeError ?)
            ExprAst::BinOp { lhs, op, rhs } => get_binop_type(lhs.as_ref(), *op, rhs.as_ref(), vars),
            ExprAst::InfixOp { op: _, val } => get_infix_type(val.as_ref(), vars),
            ExprAst::PostfixOp { val, op: _ } => get_postfix_type(val.as_ref(), vars),
            ExprAst::FunctionCall { fun, args: _ } => get_function_call_type(fun.as_ref(), vars),
            ExprAst::String(_) => Type::Ptr(Box::new(Type::Integer(Sign::Signed, IntegerType::Char))),
        }
    }
}