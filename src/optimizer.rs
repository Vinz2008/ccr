use std::mem;

use crate::{lexer::Operator, options::OptLevel, parser::{BinaryOp, ExprAst, StatementAst, TopLevelAst}};

// for statements like 1; to be removed
fn expr_has_no_side_effects(e : &ExprAst) -> bool {
    match e {
        ExprAst::BinOp { lhs: _, op, rhs: _ } => *op != BinaryOp::Equal, 
        ExprAst::Number(_) | ExprAst::Char(_) | ExprAst::VarUse(_) | ExprAst::String(_) | ExprAst::Sizeof(_) => true,
        ExprAst::FunctionCall { .. } | ExprAst::InfixOp { .. } | ExprAst::PostfixOp { .. } => false,
    }
}

fn optimize_binop(binop_expr : &mut ExprAst){
    let res = match binop_expr {
        ExprAst::BinOp { lhs, op, rhs } => {
            let op = *op;
            optimize_expr(rhs);
            if op == BinaryOp::Equal {
                return;
            }
            optimize_expr(lhs);
            match (lhs.as_ref(), rhs.as_ref()){
                (&ExprAst::Number(nb_lhs), &ExprAst::Number(nb_rhs)) => {
                    let res = match op {
                        BinaryOp::Plus => nb_lhs + nb_rhs,
                        BinaryOp::Minus => nb_lhs - nb_rhs,
                        BinaryOp::Mult => nb_lhs * nb_rhs,
                        BinaryOp::Div => nb_lhs / nb_rhs, // TODO : check for division by 0 here
                        BinaryOp::Rem => nb_lhs % nb_rhs, // TODO : check for division by 0 here
                        BinaryOp::Cmp => (nb_lhs == nb_rhs) as i128,
                        BinaryOp::Lower => (nb_lhs < nb_rhs) as i128,
                        BinaryOp::Upper => (nb_lhs > nb_rhs) as i128,
                        BinaryOp::LowerEqual => (nb_lhs <= nb_rhs) as i128,
                        BinaryOp::UpperEqual => (nb_lhs >= nb_rhs) as i128,
                        BinaryOp::Equal => unreachable!(),
                    };
                    Some(res)
                },
                _ => None,
            }
        }
        _ => unreachable!(),
    };
    if let Some(res) = res {
        *binop_expr = ExprAst::Number(res);
    }
}

fn optimize_infix_op(infix_expr : &mut ExprAst){
    let res = match infix_expr {
        ExprAst::InfixOp { op, val } => {
            let op = *op;
            optimize_expr(val);
            match val.as_ref() {
                &ExprAst::Number(val) => {
                    let res = match op {
                        Operator::Minus => -val,
                        _ => unreachable!(),
                    };
                    Some(res)
                },
                _ => None,
            }
        }
        _ => unreachable!(),
    };
    if let Some(res) = res {
        *infix_expr = ExprAst::Number(res);
    }
}

fn optimize_expr(expr : &mut ExprAst){
    match expr {
        ExprAst::Number(_) | ExprAst::Char(_) => {},
        ExprAst::BinOp { .. } => optimize_binop(expr),
        ExprAst::InfixOp { .. } => optimize_infix_op(expr),
        ExprAst::PostfixOp { val, op: _ } => {
            optimize_expr(val.as_mut());
        },
        ExprAst::VarUse(_) | ExprAst::String(_) => {},
        ExprAst::FunctionCall { fun: _, args } => {
            for arg in args {
                optimize_expr(arg);
            }
        }
        ExprAst::Sizeof(_) => {}, // TODO : should I in optimizations also replace the sizeof to help optimize cases like 5 * sizeof(int)
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
enum StatementChange {
    DoNothing,
    Remove,
}

// TODO : change this ? (see comment on top of optimize_function)
fn remove_statement(statement : &mut StatementAst){
    *statement = StatementAst::Scope { body: Box::new([]) }
}

fn optimize_statement(statement : &mut StatementAst) -> StatementChange {
    match statement {
        StatementAst::Expr(e) => {
            if expr_has_no_side_effects(e){
                return StatementChange::Remove;
            }
        }
        StatementAst::Return(e) => optimize_expr(e),
        StatementAst::Var { name: _, var_type: _, val } => optimize_expr(val),
        StatementAst::If { condition, if_body, else_body } => {
            optimize_expr(condition);
            let if_change = optimize_statement(if_body);
            let else_change = if let Some(else_body) = else_body {
                optimize_statement(else_body)
            } else {
                StatementChange::DoNothing
            };
            if else_change == StatementChange::Remove {
                *else_body = None;
            }
            if if_change == StatementChange::Remove && else_body.is_none() && expr_has_no_side_effects(condition){
                return StatementChange::Remove;
            }
            if if_change == StatementChange::Remove {
                remove_statement(if_body);
            }
        }
        StatementAst::While { condition, body } => {
            optimize_expr(condition);
            let body_change = optimize_statement(body);
            if body_change == StatementChange::Remove && expr_has_no_side_effects(condition){
                return StatementChange::Remove;
            }
            if body_change == StatementChange::Remove {
                remove_statement(body);
            }
        }
        StatementAst::For { init, condition, change, for_body } => {
            let init_change = optimize_statement(init);
            optimize_expr(condition);
            optimize_expr(change);
            let for_change = optimize_statement(for_body);
            if init_change == StatementChange::Remove && expr_has_no_side_effects(condition) && expr_has_no_side_effects(change) && for_change == StatementChange::Remove {
                return StatementChange::Remove;
            }
            if init_change == StatementChange::Remove {
                remove_statement(init);
            }
            if for_change == StatementChange::Remove {
                remove_statement(for_body);
            }
        }
        StatementAst::Scope { body } => {
            let mut is_all_removed = true;
            let mut idx_to_remove = Vec::new();
            for (idx, s) in body.iter_mut().enumerate() {
                let statement_change = optimize_statement(s);
                if statement_change != StatementChange::Remove {
                    is_all_removed = false;
                    idx_to_remove.push(idx);
                }
            }
            if is_all_removed {
                return StatementChange::Remove;
            }
            let mut body_vec = mem::take(body).into_vec();
            for &idx in idx_to_remove.iter().rev() {
                body_vec.remove(idx);
            }
            *body = body_vec.into_boxed_slice();
        }
    }
    StatementChange::DoNothing
}

// TODO : should I transform the ast to use vecs instead of Box<[]> to be able to remove/push from the list of statements ? or should I just do the round trip box[] -> vec -> box[] for it ?
fn optimize_function(statements : &mut Box<[StatementAst]>){
    let mut idx_to_remove = Vec::new();
    for (idx, statement) in statements.iter_mut().enumerate() {
        let statement_change = optimize_statement(statement);
        if statement_change == StatementChange::Remove {
            idx_to_remove.push(idx);
        }
    }
    
    if !idx_to_remove.is_empty(){
        let mut statements_vec = mem::take(statements).into_vec();
        for &idx in idx_to_remove.iter().rev(){
            statements_vec.remove(idx);
        }
        *statements = statements_vec.into_boxed_slice();
    }
    
}

pub(crate) fn optimize(ast : &mut Vec<TopLevelAst>, opt_level : OptLevel){
    if opt_level == OptLevel::O0 {
        return;
    }
    for a in ast {
        match a {
            TopLevelAst::FuncProto { .. } => {},
            TopLevelAst::Function { name: _, args: _, body, return_type: _ } => {
                optimize_function(body);
            }
        }
    }
}