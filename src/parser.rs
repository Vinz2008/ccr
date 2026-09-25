use std::collections::VecDeque;

use crate::lexer::{BinOp, Token, Type};

// TODO : make it flat ? (is more optimized, but would complicated mutating it for peep hole opts)
#[derive(Debug)]
pub(crate) enum ExprAst {
    Number(u128),
    BinOp {
        lhs : Box<ExprAst>,
        op : BinOp,
        rhs : Box<ExprAst>,
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
    Return(Box<ExprAst>),
    Var {
        name: String,
        var_type: Type,
        val : ExprAst,
    }
}

#[derive(Debug)]
pub(crate) struct Arg {
    pub name : String,
    pub arg_type : Type,
}

#[derive(Debug)]
pub(crate) enum TopLevelAst {
    Function {
        name: String,
        args : Vec<Arg>,
        body: Vec<StatementAst>,
        return_type : Type,
    }
}

// TODO : add helper function like eat token with an expected token or not ?

fn parse_primary(tokens : &mut VecDeque<Token>) -> ExprAst {
    let t = tokens.pop_front().unwrap(); // TODO : better error handling
    match t {
        Token::Number(nb) => ExprAst::Number(nb),
        Token::Identifier(ident) => ExprAst::VarUse(ident.into_boxed_str()),
        _ => panic!("Unknown token {:?}", t),
    }
}

fn parse_function_call(tokens : &mut VecDeque<Token>) -> ExprAst {
    let expr = parse_primary(tokens);
    if let Some(Token::LeftParen) = tokens.front(){
        tokens.pop_front().unwrap(); // eat (
        let mut is_first = true;
        let mut args = Vec::new();
        while let Some(t) = tokens.front() && !matches!(t, Token::RightParen) {
            if is_first {
                is_first = false;
            } else {
                tokens.pop_front().unwrap(); // eat ,
            }
            let arg = parse_expr(tokens);
            args.push(arg);
        }
        tokens.pop_front().unwrap(); // eat )
        ExprAst::FunctionCall { fun: Box::new(expr), args: args.into_boxed_slice() }
    } else {
        expr
    }
}

fn get_prec(binop : BinOp) -> u8 {
    match binop {
        BinOp::Plus | BinOp::Minus => 1,
        BinOp::Mult | BinOp::Div => 2,
    }
}

fn parse_binop(tokens : &mut VecDeque<Token>, mut lhs : ExprAst, min_prec : u8) -> ExprAst {
    let mut peek_tok = tokens.front().cloned();
    while let Some(Token::BinOp(binop)) = peek_tok && get_prec(binop) >= min_prec {
        let op = binop;
        let op_prec = get_prec(op);
        tokens.pop_front().unwrap();
        let mut rhs = parse_primary(tokens);
        peek_tok = tokens.front().cloned();

        // TODO : add the case for right associative op with precedence equal to op, also need to change the +1 in parse_binop call to be conditional (see https://en.wikipedia.org/wiki/Operator-precedence_parser)
        while let Some(Token::BinOp(binop)) = peek_tok && get_prec(binop) > op_prec {
            rhs = parse_binop(tokens, rhs, op_prec + 1);
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
    let lhs = parse_function_call(tokens);
    parse_binop(tokens, lhs, 0)
}

fn parse_var_decl(tokens : &mut VecDeque<Token>, var_type : Type) -> StatementAst {
    let ident = tokens.pop_front().unwrap();
    let ident_str = match ident {
        Token::Identifier(ident) => ident,
        _ => panic!("expected identifier"),
    };

    tokens.pop_front().unwrap(); // eat =

    let val = parse_expr(tokens);

    StatementAst::Var { 
        name: ident_str, 
        var_type,
        val,
    }
}

fn parse_statement(tokens : &mut VecDeque<Token>) -> StatementAst {
    let t = tokens.pop_front().unwrap(); // TODO : better error handling
    dbg!(&t);
    let statement = match t {
        Token::Return => StatementAst::Return(Box::new(parse_expr(tokens))),
        Token::Type(t) => parse_var_decl(tokens, t),
        _ => StatementAst::Expr(parse_expr(tokens)),
    };
    match tokens.front(){
        Some(Token::SemiColon) => {
            tokens.pop_front().unwrap();
        },
        _ => panic!("missing semicolon"),
    }
    statement
}

// TODO : add global var support
fn parse_top_level_decl(tokens : &mut VecDeque<Token>, t : Type) -> TopLevelAst {
    let ident = tokens.pop_front().unwrap();
    let ident_str = match ident {
        Token::Identifier(ident) => ident,
        _ => panic!("expected identifier"),
    };
    tokens.pop_front().unwrap(); // eat (
    let mut args = Vec::new();
    let mut is_first = true;
    while let Some(t) = tokens.front() && !matches!(t, Token::RightParen) {
        if is_first {
            is_first = false;
        } else {
            tokens.pop_front().unwrap(); // eat ,
        }
        let type_tok = tokens.pop_front().unwrap();
        let arg_type = match type_tok {
            Token::Type(t) => t,
            _ => panic!("expected type"),
        };
        let arg_name_tok = tokens.pop_front().unwrap();
        let arg_name = match arg_name_tok {
            Token::Identifier(arg_name) => arg_name,
            _ => panic!("expected identifier"),
        };
        args.push(Arg { name: arg_name, arg_type });
    }
    tokens.pop_front().unwrap(); // eat )
    tokens.pop_front().unwrap(); // eat {

    let mut statements = Vec::new();
    while let Some(t) = tokens.front() && !matches!(t, Token::RightBrace) {
        statements.push(parse_statement(tokens));
    }
    tokens.pop_front().unwrap(); // eat }
    TopLevelAst::Function { 
        name: ident_str, 
        body: statements,
        return_type: t,
        args,
    }
}

fn parse_top_level(tokens : &mut VecDeque<Token>) -> TopLevelAst {
    let t = tokens.pop_front().unwrap(); // TODO : better error handling
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