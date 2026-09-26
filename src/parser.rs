use std::collections::VecDeque;

use enum_tag::EnumTag;

use crate::{lexer::{BinOp, Token, TokenTag}, types::Type};

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

// TODO : replace all the Strings and Vec in these by Box<str> and Box slices


#[derive(Debug)]
pub(crate) enum StatementAst {
    Expr(ExprAst),
    Return(ExprAst),
    Var {
        name: String,
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

// TODO : better error handling for these
fn eat_token(tokens : &mut VecDeque<Token>, token_type : TokenTag) -> Token {
    let tok = tokens.pop_front().unwrap();
    if token_type != tok.tag() {
        panic!("wrong token : {:?}, expected : {:?}", tok.tag(), token_type);
    }
    tok
}

#[inline(always)]
fn pass_token(tokens : &mut VecDeque<Token>) -> Token {
    tokens.pop_front().unwrap()
}

fn parse_primary(tokens : &mut VecDeque<Token>) -> ExprAst {
    let t = pass_token(tokens);
    match t {
        Token::Number(nb) => ExprAst::Number(nb),
        Token::Identifier(ident) => ExprAst::VarUse(ident.into_boxed_str()),
        _ => panic!("Unknown token {:?}", t),
    }
}

fn parse_function_call(tokens : &mut VecDeque<Token>) -> ExprAst {
    let expr = parse_primary(tokens);
    if let Some(Token::LeftParen) = tokens.front(){
        eat_token(tokens, TokenTag::LeftParen);
        let mut is_first = true;
        let mut args = Vec::new();
        while let Some(t) = tokens.front() && !matches!(t, Token::RightParen) {
            if is_first {
                is_first = false;
            } else {
                eat_token(tokens, TokenTag::Colon);
            }
            let arg = parse_expr(tokens);
            args.push(arg);
        }
        eat_token(tokens, TokenTag::RightParen);
        ExprAst::FunctionCall { fun: Box::new(expr), args: args.into_boxed_slice() }
    } else {
        expr
    }
}

fn get_prec(binop : BinOp) -> u8 {
    match binop {
        BinOp::Cmp => 1,
        BinOp::Plus | BinOp::Minus => 2,
        BinOp::Mult | BinOp::Div => 3,
    }
}

fn parse_binop(tokens : &mut VecDeque<Token>, mut lhs : ExprAst, min_prec : u8) -> ExprAst {
    let mut peek_tok = tokens.front().cloned();
    while let Some(Token::BinOp(binop)) = peek_tok && get_prec(binop) >= min_prec {
        pass_token(tokens);
        let op = binop;
        let op_prec = get_prec(op);
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
    let ident = eat_token(tokens, TokenTag::Identifier);
    let ident_str = match ident {
        Token::Identifier(ident) => ident,
        _ => unreachable!(),
    };

    eat_token(tokens, TokenTag::Equal);

    let val = parse_expr(tokens);

    StatementAst::Var { 
        name: ident_str, 
        var_type,
        val,
    }
}

fn parse_if(tokens : &mut VecDeque<Token>) -> StatementAst {
    eat_token(tokens, TokenTag::LeftParen);
    let condition = parse_expr(tokens);
    eat_token(tokens, TokenTag::RightParen);

    eat_token(tokens, TokenTag::LeftBrace);
    let mut if_body = Vec::new();
    while let Some(t) = tokens.front() && !matches!(t, Token::RightBrace) {
        if_body.push(parse_statement(tokens));
    }
    eat_token(tokens, TokenTag::RightBrace);
    let mut else_body = None;
    if let Some(Token::Else) = tokens.front() {
        eat_token(tokens, TokenTag::Else);
        eat_token(tokens, TokenTag::LeftBrace);
        let mut else_statements = Vec::new();
        while let Some(t) = tokens.front() && !matches!(t, Token::RightBrace) {
            else_statements.push(parse_statement(tokens));
        }
        eat_token(tokens, TokenTag::RightBrace);
        else_body = Some(else_statements.into_boxed_slice());
    }
    
    
    
    StatementAst::If { 
        condition, 
        if_body: if_body.into_boxed_slice(), 
        else_body: else_body, 
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
        match tokens.front(){
            Some(Token::SemiColon) => {
                pass_token(tokens);
            },
            _ => panic!("missing semicolon"),
        }
    }
    statement
}

// TODO : add global var support
fn parse_top_level_decl(tokens : &mut VecDeque<Token>, t : Type) -> TopLevelAst {
    let ident = eat_token(tokens, TokenTag::Identifier);
    let ident_str = match ident {
        Token::Identifier(ident) => ident,
        _ => unreachable!(),
    };
    eat_token(tokens, TokenTag::LeftParen);
    let mut args = Vec::new();
    let mut is_first = true;
    while let Some(t) = tokens.front() && !matches!(t, Token::RightParen) {
        if is_first {
            is_first = false;
        } else {
            eat_token(tokens, TokenTag::Colon);
        }
        let type_tok = eat_token(tokens, TokenTag::Type);
        let arg_type = match type_tok {
            Token::Type(t) => t,
            _ => unreachable!(),
        };
        let arg_name_tok = eat_token(tokens, TokenTag::Identifier);
        let arg_name = match arg_name_tok {
            Token::Identifier(arg_name) => arg_name,
            _ => unreachable!(),
        };
        args.push(Arg { name: arg_name, arg_type });
    }
    eat_token(tokens, TokenTag::RightParen);
    eat_token(tokens, TokenTag::LeftBrace);

    let mut statements = Vec::new();
    while let Some(t) = tokens.front() && !matches!(t, Token::RightBrace) {
        statements.push(parse_statement(tokens));
    }
    eat_token(tokens, TokenTag::RightBrace);
    TopLevelAst::Function { 
        name: ident_str, 
        body: statements,
        return_type: t,
        args,
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