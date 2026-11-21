use std::collections::VecDeque;

use crate::{lexer::{Operator, Token}, types::Type};

#[derive(Debug)]
pub(crate) enum ParserError {
    WrongTok {
        got : Token,
        expected : Box<str>,
    },
    UnexpectedTok(Token),
    WrongUnaryOp(Operator),
    WrongBinOp(Operator),
    UnexpectedEof,
}

// TODO : refactor this ? (maybe make the lexer op a enum of these ?)

#[derive(Debug, Clone, Copy)]
pub(crate) enum PostfixOp {
    PlusPlus,
    MinusMinus,
}

impl TryFrom<Operator> for PostfixOp {
    type Error = ();

    fn try_from(value: Operator) -> Result<Self, Self::Error> {
        let postfix_op = match value {
            Operator::PlusPlus => PostfixOp::PlusPlus,
            Operator::MinusMinus => PostfixOp::MinusMinus,
            _ => return Err(()),
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
    type Error = ParserError;

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
            _ => return Err(ParserError::WrongBinOp(value)),
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

macro_rules! eat_token {
    ($expression:expr, $pattern:pat) => {
        {
            let tokens : &mut VecDeque<Token> = $expression;
            let tok = tokens.pop_front().ok_or(ParserError::UnexpectedEof)?;
            match tok {
                $pattern => tok,
                _ => return Err(ParserError::WrongTok { got : tok, expected : Box::from(stringify!($pattern))}),
            }
        }
    };
    ($expression:expr, $pattern:pat, $expr_ret:expr) => {
        {
            let tokens : &mut VecDeque<Token> = $expression;
            let tok = tokens.pop_front().ok_or(ParserError::UnexpectedEof)?;
            match tok {
                $pattern => $expr_ret,
                _ => return Err(ParserError::WrongTok { got : tok, expected : Box::from(stringify!($pattern))}),
            }
        } 
    }
}

#[inline(always)]
fn pass_token(tokens : &mut VecDeque<Token>) -> Result<Token, ParserError> {
    tokens.pop_front().ok_or(ParserError::UnexpectedEof)
}

fn parse_primary(tokens : &mut VecDeque<Token>) -> Result<ExprAst, ParserError> {
    let t = pass_token(tokens)?;
    let e = match t {
        Token::Number(nb) => ExprAst::Number(nb),
        Token::Identifier(ident) => ExprAst::VarUse(ident),
        Token::Char(c) => ExprAst::Char(c),
        Token::LeftParen => {
            let expr = parse_expr(tokens)?;
            eat_token!(tokens, Token::RightParen);
            expr
        }
        Token::String(str) => ExprAst::String(str),
        _ => return Err(ParserError::UnexpectedTok(t)),
    };
    Ok(e)
}

fn parse_postfix(tokens : &mut VecDeque<Token>) -> Result<ExprAst, ParserError> {
    let expr = parse_primary(tokens)?;
    let e = match tokens.front(){
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
                let arg = parse_expr(tokens)?;
                args.push(arg);
            }
            eat_token!(tokens, Token::RightParen);
            ExprAst::FunctionCall { fun: Box::new(expr), args: args.into_boxed_slice() }
        }
        Some(&Token::Operator(op)) => {
            let postfix_op = match op.try_into(){
                Ok(op) => op,
                Err(_) => return Ok(expr),
            };
            pass_token(tokens)?;
            ExprAst::PostfixOp { val: Box::new(expr), op: postfix_op }
        }
        _ => expr,
    };
    Ok(e)
}

fn parse_unary(tokens : &mut VecDeque<Token>) -> Result<ExprAst, ParserError> {
    if let Some(Token::Operator(op)) = tokens.front().cloned() {
        match op {
            Operator::Minus => {},
            _ => return Err(ParserError::WrongUnaryOp(op)),
        }
        pass_token(tokens)?;
        let expr = parse_postfix(tokens)?;
        Ok(ExprAst::InfixOp { op, val: Box::new(expr) })
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

fn parse_binop(tokens : &mut VecDeque<Token>, mut lhs : ExprAst, min_prec : u8) -> Result<ExprAst, ParserError> {
    let mut peek_tok = tokens.front().cloned();
    while let Some(Token::Operator(op)) = peek_tok {
        let binop = op.try_into()?;
        if get_prec(binop) < min_prec {
            break;
        }
        pass_token(tokens)?;
        let op_prec = get_prec(binop);
        let mut rhs = parse_unary(tokens)?;
        peek_tok = tokens.front().cloned();

        while let Some(Token::Operator(op)) = peek_tok {
            let binop = op.try_into()?;
            if get_prec(binop) <= op_prec && (!is_right_associative(binop) || get_prec(binop) != op_prec){
                break;
            }
            let increment = if get_prec(binop) > op_prec {
                1
            } else {
                0
            };
            rhs = parse_binop(tokens, rhs, op_prec + increment)?;
            peek_tok = tokens.front().cloned();
        }
        lhs = ExprAst::BinOp { 
            lhs: Box::new(lhs), 
            op: binop,
            rhs: Box::new(rhs) 
        };
    }
    Ok(lhs)
}

fn parse_expr(tokens : &mut VecDeque<Token>) -> Result<ExprAst, ParserError> {
    let lhs = parse_unary(tokens)?;
    parse_binop(tokens, lhs, 0)
}

fn parse_var_decl(tokens : &mut VecDeque<Token>, var_type : Type) -> Result<StatementAst, ParserError> {
    let ident_str = eat_token!(tokens, Token::Identifier(s), s);

    eat_token!(tokens, Token::Operator(Operator::Equal));

    let val = parse_expr(tokens)?;

    Ok(StatementAst::Var { 
        name: ident_str, 
        var_type,
        val,
    })
}

fn parse_scope(tokens : &mut VecDeque<Token>) -> Result<StatementAst, ParserError> {
    eat_token!(tokens, Token::LeftBrace);
    let mut body = Vec::new();
    while let Some(t) = tokens.front() && !matches!(t, Token::RightBrace) {
        body.push(parse_statement(tokens)?);
    }
    eat_token!(tokens, Token::RightBrace);
    Ok(StatementAst::Scope { 
        body: body.into_boxed_slice(),
    })
}

fn parse_if(tokens : &mut VecDeque<Token>) -> Result<StatementAst, ParserError> {
    eat_token!(tokens, Token::If);
    eat_token!(tokens, Token::LeftParen);
    let condition = parse_expr(tokens)?;
    eat_token!(tokens, Token::RightParen);

    let if_body = parse_statement(tokens)?;
    
    let mut else_body = None;
    if let Some(Token::Else) = tokens.front() {
        eat_token!(tokens, Token::Else);
        let body = parse_statement(tokens)?;
        else_body = Some(Box::new(body));
    }
    
    Ok(StatementAst::If { 
        condition, 
        if_body: Box::new(if_body), 
        else_body, 
    })
}

fn parse_while(tokens : &mut VecDeque<Token>) -> Result<StatementAst, ParserError> {
    eat_token!(tokens, Token::While);
    eat_token!(tokens, Token::LeftParen);
    let condition = parse_expr(tokens)?;
    eat_token!(tokens, Token::RightParen);

    let while_body = parse_statement(tokens)?;

    Ok(StatementAst::While { 
        condition, 
        body: Box::new(while_body),
    })
}

fn parse_for(tokens : &mut VecDeque<Token>) -> Result<StatementAst, ParserError> {
    eat_token!(tokens, Token::For);
    eat_token!(tokens, Token::LeftParen);

    let init = parse_statement(tokens)?;
    dbg!(&tokens);
    let condition = parse_expr(tokens)?;
    eat_token!(tokens, Token::SemiColon);
    let change = parse_expr(tokens)?;
    eat_token!(tokens, Token::RightParen);
    let for_body = parse_statement(tokens)?;
    
    Ok(StatementAst::For { 
        init: Box::new(init), 
        condition,
        change ,
        for_body: Box::new(for_body),
    })
}

fn parse_statement(tokens : &mut VecDeque<Token>) -> Result<StatementAst, ParserError> {
    let t = tokens.front().ok_or(ParserError::UnexpectedEof)?;
    dbg!(&t);
    let mut need_semicolon = true;
    let statement = match t {
        Token::Return => {
            pass_token(tokens)?;
            StatementAst::Return(parse_expr(tokens)?)
        },
        Token::Type(t) => {
            let t = t.clone();
            pass_token(tokens)?;
            parse_var_decl(tokens, t)?
        },
        Token::If => {
            need_semicolon = false;
            parse_if(tokens)?
        },
        Token::While => {
            need_semicolon = false;
            parse_while(tokens)?
        }
        Token::For => {
            need_semicolon = false;
            parse_for(tokens)?
        }
        Token::LeftBrace => {
            need_semicolon = false;
            parse_scope(tokens)?
        }
        _ => StatementAst::Expr(parse_expr(tokens)?),
    };
    if need_semicolon {
        eat_token!(tokens, Token::SemiColon);
    }
    Ok(statement)
}

// TODO : add global var support
fn parse_top_level_decl(tokens : &mut VecDeque<Token>, t : Type) -> Result<TopLevelAst, ParserError> {
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
            return Ok(TopLevelAst::FuncProto { 
                name: ident_str,
                return_type: t, 
                args: args.into_boxed_slice(), 
            });
        }
        Some(_) => {
            eat_token!(tokens, Token::LeftBrace);
        }
        None => return Err(ParserError::UnexpectedEof),
    }

    let mut statements = Vec::new();
    while let Some(t) = tokens.front() && !matches!(t, Token::RightBrace) {
        statements.push(parse_statement(tokens)?);
    }
    eat_token!(tokens, Token::RightBrace);
    Ok(TopLevelAst::Function { 
        name: ident_str, 
        body: statements.into_boxed_slice(),
        return_type: t,
        args: args.into_boxed_slice(),
    })
}

fn parse_top_level(tokens : &mut VecDeque<Token>) -> Result<TopLevelAst, ParserError> {
    let t = pass_token(tokens)?;
    match t {
        Token::Type(t) => {
            parse_top_level_decl(tokens, t)
        },
        _ => Err(ParserError::UnexpectedTok(t)),
    }
}

pub(crate) fn parse(mut tokens : VecDeque<Token>) -> Result<Vec<TopLevelAst>, ParserError> {
    let mut top_level = Vec::new();
    while !tokens.is_empty(){
        top_level.push(parse_top_level(&mut tokens)?);
    }
    Ok(top_level)
}