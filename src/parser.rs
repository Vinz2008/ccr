use std::collections::VecDeque;

use crate::{lexer::{Operator, Token}, types::Type};

// TODO : refactor this ? (maybe make the lexer op a enum of these ?)

#[derive(Debug, Clone, Copy)]
pub(crate) enum PostfixOp {
    PlusPlus,
    MinusMinus,
}

#[derive(Debug)]
pub(crate) struct OperatorError(Operator);

impl TryFrom<Operator> for PostfixOp {
    type Error = OperatorError; // TODO : better error ?

    fn try_from(value: Operator) -> Result<Self, Self::Error> {
        let postfix_op = match value {
            Operator::PlusPlus => PostfixOp::PlusPlus,
            Operator::MinusMinus => PostfixOp::MinusMinus,
            _ => return Err(OperatorError(value)),
        };
        Ok(postfix_op)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BinaryOp {
    Plus,
    Minus,
    Mult,
    Div,
    Rem,
    Cmp, // ==
    Equal, // =
    Lower, // <
    Upper,
    LowerEqual,
    UpperEqual,
}

impl TryFrom<Operator> for BinaryOp {
    type Error = (); // TODO : better error ?

    fn try_from(value: Operator) -> Result<Self, Self::Error> {
        let binary_op = match value {
            Operator::Plus => BinaryOp::Plus,
            Operator::Minus => BinaryOp::Minus,
            Operator::Mult => BinaryOp::Mult,
            Operator::Div => BinaryOp::Div,
            Operator::Rem => BinaryOp::Rem,
            Operator::Cmp => BinaryOp::Cmp,
            Operator::Equal => BinaryOp::Equal,
            Operator::Lower => BinaryOp::Lower,
            Operator::Upper => BinaryOp::Upper,
            Operator::LowerEqual => BinaryOp::LowerEqual,
            Operator::UpperEqual => BinaryOp::UpperEqual,
            _ => return Err(()),
        };
        Ok(binary_op)
    }
}

// TODO : make it flat ? (is more optimized, but would complicated mutating it for peep hole opts)
#[derive(Debug)]
pub(crate) enum ExprAst {
    Number(u128),
    Char(char),
    BinOp {
        lhs : Box<ExprAst>,
        op : BinaryOp,
        rhs : Box<ExprAst>,
    },
    InfixOp {
        op : Operator,
        val : Box<ExprAst>,
    },
    PostfixOp {
        val : Box<ExprAst>,
        op : PostfixOp,
    },
    VarUse(Box<str>),
    String(Box<str>),
    FunctionCall {
        fun : Box<ExprAst>,
        args : Box<[ExprAst]>,
    }
}


// TODO : separate this between real statements (the expresions that can be used here : if (a) [STATEMENT] and the block items, that include additionaly var decls, which can't be used in the if body without a block)

#[derive(Debug)]
pub(crate) enum StatementAst {
    Expr(ExprAst),
    Return(ExprAst),
    Var {
        name: Box<str>,
        var_type: Type,
        val : ExprAst,
    },
    // TODO : how to implement else if ? in the ast ? or as sugaring as if else in one another ?
    If {
        condition : ExprAst,
        if_body : Box<StatementAst>,
        else_body : Option<Box<StatementAst>>,
    },
    While {
        condition : ExprAst,
        body : Box<StatementAst>,
    },
    For {
        init : Box<StatementAst>, // TODO : this should be either an init or expr
        condition : ExprAst,
        change : ExprAst,
        for_body : Box<StatementAst>,
    },
    Scope {
        body : Box<[StatementAst]>,
    }
}

#[derive(Debug)]
pub(crate) struct Arg {
    pub name : Box<str>,
    pub arg_type : Type,
}

#[derive(Debug)]
pub(crate) enum TopLevelAst {
    Function {
        name: Box<str>,
        args : Box<[Arg]>,
        body: Box<[StatementAst]>,
        return_type : Type,
    },
    FuncProto {
        name: Box<str>,
        args : Box<[Arg]>,
        return_type : Type,
    }
}

// TODO : better error handling for these


macro_rules! eat_token {
    ($expression:expr, $pattern:pat) => {
        {
            let tokens : &mut VecDeque<Token> = $expression;
            let tok = tokens.pop_front().unwrap();
            match tok {
                $pattern => tok,
                _ => panic!("wrong token : {:?}, expected : {}", tok, stringify!($pattern))
            }
        }
    };
    ($expression:expr, $pattern:pat, $expr_ret:expr) => {
        {
            let tokens : &mut VecDeque<Token> = $expression;
            let tok = tokens.pop_front().unwrap();
            match tok {
                $pattern => $expr_ret,
                _ => panic!("wrong token : {:?}, expected : {}", tok, stringify!($pattern))
            }
        } 
    }
}

#[inline(always)]
fn pass_token(tokens : &mut VecDeque<Token>) -> Token {
    tokens.pop_front().unwrap()
}

fn parse_primary(tokens : &mut VecDeque<Token>) -> ExprAst {
    let t = pass_token(tokens);
    match t {
        Token::Number(nb) => ExprAst::Number(nb),
        Token::Identifier(ident) => ExprAst::VarUse(ident),
        Token::Char(c) => ExprAst::Char(c),
        Token::LeftParen => {
            let expr = parse_expr(tokens);
            eat_token!(tokens, Token::RightParen);
            expr
        }
        Token::String(str) => ExprAst::String(str),
        _ => panic!("Unknown token {:?}", t),
    }
}

fn parse_postfix(tokens : &mut VecDeque<Token>) -> ExprAst {
    let expr = parse_primary(tokens);
    match tokens.front(){
        Some(Token::LeftParen) => {
            eat_token!(tokens, Token::LeftParen);
            let mut is_first = true;
            let mut args = Vec::new();
            while let Some(t) = tokens.front() && !matches!(t, Token::RightParen) {
                if is_first {
                    is_first = false;
                } else {
                    eat_token!(tokens, Token::Colon);
                }
                let arg = parse_expr(tokens);
                args.push(arg);
            }
            eat_token!(tokens, Token::RightParen);
            ExprAst::FunctionCall { fun: Box::new(expr), args: args.into_boxed_slice() }
        }
        Some(&Token::Operator(op)) => {
            let postfix_op = match op.try_into(){
                Ok(op) => op,
                Err(_) => return expr,
            };
            pass_token(tokens);
            ExprAst::PostfixOp { val: Box::new(expr), op: postfix_op }
        }
        _ => expr,
    }
}

fn parse_unary(tokens : &mut VecDeque<Token>) -> ExprAst {
    if let Some(Token::Operator(op)) = tokens.front().cloned() {
        match op {
            Operator::Minus => {},
            _ => panic!("wrong unary operator"),
        }
        pass_token(tokens);
        let expr = parse_postfix(tokens);
        ExprAst::InfixOp { op, val: Box::new(expr) }
    } else {
        parse_postfix(tokens)
    }
}

fn get_prec(binop : BinaryOp) -> u8 {
    match binop {
        BinaryOp::Equal => 1,
        BinaryOp::Cmp => 2,
        BinaryOp::Lower | BinaryOp::Upper | BinaryOp::LowerEqual | BinaryOp::UpperEqual => 3,
        BinaryOp::Plus | BinaryOp::Minus => 4,
        BinaryOp::Mult | BinaryOp::Div | BinaryOp::Rem => 5,
    }
}

fn is_right_associative(binop : BinaryOp) -> bool {
    match binop {
        BinaryOp::Equal => true,
        BinaryOp::Plus | BinaryOp::Minus | BinaryOp::Mult | BinaryOp::Div 
        | BinaryOp::Rem | BinaryOp::Cmp | BinaryOp::Lower | BinaryOp::Upper | BinaryOp::LowerEqual | BinaryOp::UpperEqual => false,
    }
}

fn parse_binop(tokens : &mut VecDeque<Token>, mut lhs : ExprAst, min_prec : u8) -> ExprAst {
    let mut peek_tok = tokens.front().cloned();
    while let Some(Token::Operator(op)) = peek_tok {
        let binop = op.try_into().unwrap(); // TODO : better error handling
        if get_prec(binop) < min_prec {
            break;
        }
        pass_token(tokens);
        let op_prec = get_prec(binop);
        let mut rhs = parse_unary(tokens);
        peek_tok = tokens.front().cloned();

        while let Some(Token::Operator(op)) = peek_tok {
            let binop = op.try_into().unwrap();
            if get_prec(binop) <= op_prec && (!is_right_associative(binop) || get_prec(binop) != op_prec){
                break;
            }
            let increment = if get_prec(binop) > op_prec {
                1
            } else {
                0
            };
            rhs = parse_binop(tokens, rhs, op_prec + increment);
            peek_tok = tokens.front().cloned();
        }
        lhs = ExprAst::BinOp { 
            lhs: Box::new(lhs), 
            op: binop,
            rhs: Box::new(rhs) 
        };
    }
    lhs
}

fn parse_expr(tokens : &mut VecDeque<Token>) -> ExprAst {
    let lhs = parse_unary(tokens);
    parse_binop(tokens, lhs, 0)
}

fn parse_var_decl(tokens : &mut VecDeque<Token>, var_type : Type) -> StatementAst {
    let ident_str = eat_token!(tokens, Token::Identifier(s), s);

    eat_token!(tokens, Token::Operator(Operator::Equal));

    let val = parse_expr(tokens);

    StatementAst::Var { 
        name: ident_str, 
        var_type,
        val,
    }
}

fn parse_scope(tokens : &mut VecDeque<Token>) -> StatementAst {
    eat_token!(tokens, Token::LeftBrace);
    let mut body = Vec::new();
    while let Some(t) = tokens.front() && !matches!(t, Token::RightBrace) {
        body.push(parse_statement(tokens));
    }
    eat_token!(tokens, Token::RightBrace);
    StatementAst::Scope { 
        body: body.into_boxed_slice(),
    }
}

fn parse_if(tokens : &mut VecDeque<Token>) -> StatementAst {
    eat_token!(tokens, Token::If);
    eat_token!(tokens, Token::LeftParen);
    let condition = parse_expr(tokens);
    eat_token!(tokens, Token::RightParen);

    let if_body = parse_statement(tokens);
    
    let mut else_body = None;
    if let Some(Token::Else) = tokens.front() {
        eat_token!(tokens, Token::Else);
        let body = parse_statement(tokens);
        else_body = Some(Box::new(body));
    }
    
    StatementAst::If { 
        condition, 
        if_body: Box::new(if_body), 
        else_body, 
    }
}

fn parse_while(tokens : &mut VecDeque<Token>) -> StatementAst {
    eat_token!(tokens, Token::While);
    eat_token!(tokens, Token::LeftParen);
    let condition = parse_expr(tokens);
    eat_token!(tokens, Token::RightParen);

    let while_body = parse_statement(tokens);

    StatementAst::While { 
        condition, 
        body: Box::new(while_body),
    }
}

fn parse_for(tokens : &mut VecDeque<Token>) -> StatementAst {
    eat_token!(tokens, Token::For);
    eat_token!(tokens, Token::LeftParen);

    let init = parse_statement(tokens);
    dbg!(&tokens);
    //eat_token!(tokens, Token::SemiColon); // TODO : don't need it, because parse_statement already does it, should I change it (a aux function ?)
    let condition = parse_expr(tokens);
    eat_token!(tokens, Token::SemiColon);
    let change = parse_expr(tokens);
    eat_token!(tokens, Token::RightParen);
    let for_body = parse_statement(tokens);
    
    StatementAst::For { 
        init: Box::new(init), 
        condition,
        change ,
        for_body: Box::new(for_body),
    }
}

fn parse_statement(tokens : &mut VecDeque<Token>) -> StatementAst {
    let t = tokens.front().unwrap(); // TODO : better error handling
    dbg!(&t);
    let mut need_semicolon = true;
    let statement = match t {
        Token::Return => {
            pass_token(tokens);
            StatementAst::Return(parse_expr(tokens))
        },
        Token::Type(t) => {
            let t = t.clone();
            pass_token(tokens);
            parse_var_decl(tokens, t)
        },
        Token::If => {
            need_semicolon = false;
            parse_if(tokens)
        },
        Token::While => {
            need_semicolon = false;
            parse_while(tokens)
        }
        Token::For => {
            need_semicolon = false;
            parse_for(tokens)
        }
        Token::LeftBrace => {
            need_semicolon = false;
            parse_scope(tokens)
        }
        _ => StatementAst::Expr(parse_expr(tokens)),
    };
    if need_semicolon {
        eat_token!(tokens, Token::SemiColon);
    }
    statement
}

// TODO : add global var support
fn parse_top_level_decl(tokens : &mut VecDeque<Token>, t : Type) -> TopLevelAst {
    let ident_str = eat_token!(tokens, Token::Identifier(s), s);
    eat_token!(tokens, Token::LeftParen);
    let mut args = Vec::new();
    let mut is_first = true;
    while let Some(t) = tokens.front() && !matches!(t, Token::RightParen) {
        if is_first {
            is_first = false;
        } else {
            eat_token!(tokens, Token::Colon);
        }
        let arg_type = eat_token!(tokens, Token::Type(t), t);
        let arg_name = eat_token!(tokens, Token::Identifier(s), s);
        args.push(Arg { name: arg_name, arg_type });
    }
    eat_token!(tokens, Token::RightParen);
    match tokens.front(){
        Some(Token::SemiColon) => {
            eat_token!(tokens, Token::SemiColon);
            return TopLevelAst::FuncProto { 
                name: ident_str,
                return_type: t, 
                args: args.into_boxed_slice(), 
            };
        }
        Some(_) => {
            eat_token!(tokens, Token::LeftBrace);
        }
        None => panic!("Unexpected end after proto"),
    }

    let mut statements = Vec::new();
    while let Some(t) = tokens.front() && !matches!(t, Token::RightBrace) {
        statements.push(parse_statement(tokens));
    }
    eat_token!(tokens, Token::RightBrace);
    TopLevelAst::Function { 
        name: ident_str, 
        body: statements.into_boxed_slice(),
        return_type: t,
        args: args.into_boxed_slice(),
    }
}

fn parse_top_level(tokens : &mut VecDeque<Token>) -> TopLevelAst {
    let t = pass_token(tokens); // TODO : better error handling
    match t {
        Token::Type(t) => {
            parse_top_level_decl(tokens, t)
        },
        _ => panic!("Unknown token {:?}", t),
    }
}

pub(crate) fn parse(mut tokens : VecDeque<Token>) -> Vec<TopLevelAst> {
    let mut top_level = Vec::new();
    while !tokens.is_empty(){
        top_level.push(parse_top_level(&mut tokens));
    }
    top_level
}