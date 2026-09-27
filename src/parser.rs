use std::collections::VecDeque;

use crate::{lexer::{Operator, Token}, types::Type};

// TODO : make it flat ? (is more optimized, but would complicated mutating it for peep hole opts)
#[derive(Debug)]
pub(crate) enum ExprAst {
    Number(u128),
    Char(char),
    BinOp {
        lhs : Box<ExprAst>,
        op : Operator,
        rhs : Box<ExprAst>,
    },
    UnaryOp {
        op : Operator,
        val : Box<ExprAst>,
    },
    VarUse(Box<str>),
    FunctionCall {
        fun : Box<ExprAst>,
        args : Box<[ExprAst]>,
    }
}


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
        if_body : Box<[StatementAst]>, // TODO : make these body only one StatementAst after adding scopes
        else_body : Option<Box<[StatementAst]>>,
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


/*fn eat_token(tokens : &mut VecDeque<Token>, token_type : TokenTag) -> Token {
    let tok = tokens.pop_front().unwrap();
    if token_type != tok.tag() {
        panic!("wrong token : {:?}, expected : {:?}", tok.tag(), token_type);
    }
    tok
}*/

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
        _ => panic!("Unknown token {:?}", t),
    }
}

fn parse_function_call(tokens : &mut VecDeque<Token>) -> ExprAst {
    let expr = parse_primary(tokens);
    if let Some(Token::LeftParen) = tokens.front(){
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
    } else {
        expr
    }
}

fn parse_unary(tokens : &mut VecDeque<Token>) -> ExprAst {
    if let Some(Token::Operator(op)) = tokens.front().cloned() {
        match op {
            Operator::Minus => {},
            _ => panic!("wrong unary operator"),
        }
        pass_token(tokens);
        let expr = parse_function_call(tokens);
        ExprAst::UnaryOp { op, val: Box::new(expr) }
    } else {
        parse_function_call(tokens)
    }
}

fn get_prec(binop : Operator) -> u8 {
    match binop {
        Operator::Equal => 1,
        Operator::Cmp => 2,
        Operator::Plus | Operator::Minus => 3,
        Operator::Mult | Operator::Div => 4,
    }
}

fn is_right_associative(binop : Operator) -> bool {
    match binop {
        Operator::Equal => true,
        Operator::Plus | Operator::Minus | Operator::Mult | Operator::Div | Operator::Cmp => false,
    }
}

fn parse_binop(tokens : &mut VecDeque<Token>, mut lhs : ExprAst, min_prec : u8) -> ExprAst {
    let mut peek_tok = tokens.front().cloned();
    while let Some(Token::Operator(binop)) = peek_tok && get_prec(binop) >= min_prec {
        pass_token(tokens);
        let op = binop;
        let op_prec = get_prec(op);
        let mut rhs = parse_unary(tokens);
        peek_tok = tokens.front().cloned();

        while let Some(Token::Operator(binop)) = peek_tok && (get_prec(binop) > op_prec || (is_right_associative(binop) && get_prec(binop) == op_prec)) {
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
            op,
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

fn parse_if(tokens : &mut VecDeque<Token>) -> StatementAst {
    eat_token!(tokens, Token::LeftParen);
    let condition = parse_expr(tokens);
    eat_token!(tokens, Token::RightParen);

    eat_token!(tokens, Token::LeftBrace);
    let mut if_body = Vec::new();
    while let Some(t) = tokens.front() && !matches!(t, Token::RightBrace) {
        if_body.push(parse_statement(tokens));
    }
    eat_token!(tokens, Token::RightBrace);
    let mut else_body = None;
    if let Some(Token::Else) = tokens.front() {
        eat_token!(tokens, Token::Else);
        eat_token!(tokens, Token::LeftBrace);
        let mut else_statements = Vec::new();
        while let Some(t) = tokens.front() && !matches!(t, Token::RightBrace) {
            else_statements.push(parse_statement(tokens));
        }
        eat_token!(tokens, Token::RightBrace);
        else_body = Some(else_statements.into_boxed_slice());
    }
    
    
    
    StatementAst::If { 
        condition, 
        if_body: if_body.into_boxed_slice(), 
        else_body, 
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
            pass_token(tokens);
            parse_if(tokens)
        },
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