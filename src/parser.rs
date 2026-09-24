use std::collections::VecDeque;

use crate::lexer::{BinOp, Token};

// TODO : make it flat ? (is more optimized, but would complicated mutating it for peep hole opts)
#[derive(Debug)]
pub(crate) enum AstNode {
    Number(u128),
    BinOp {
        lhs : Box<AstNode>,
        op : BinOp,
        rhs : Box<AstNode>,
    },
    Return(Box<AstNode>),
}

fn parse_primary(tokens : &mut VecDeque<Token>) -> AstNode {
    let t = tokens.pop_front().unwrap(); // TODO : better error handling
    match t {
        Token::Number(nb) => AstNode::Number(nb),
        Token::Return => AstNode::Return(Box::new(parse_expr(tokens))),
        _ => panic!("Unknown token {:?}", t),
    }
}

fn get_prec(binop : BinOp) -> u8 {
    match binop {
        BinOp::Plus | BinOp::Minus => 1,
        BinOp::Mult | BinOp::Div => 2,
    }
}

fn parse_binop(tokens : &mut VecDeque<Token>, mut lhs : AstNode, min_prec : u8) -> AstNode {
    let mut peek_tok = tokens.front().cloned();
    while let Some(Token::BinOp(binop)) = peek_tok && get_prec(binop) >= min_prec {
        let op = binop;
        let op_prec = get_prec(op);
        tokens.pop_front();
        let mut rhs = parse_primary(tokens);
        peek_tok = tokens.front().cloned();

        // TODO : add the case for right associative op with precedence equal to op, also need to change the +1 in parse_binop call to be conditional (see https://en.wikipedia.org/wiki/Operator-precedence_parser)
        while let Some(Token::BinOp(binop)) = peek_tok && get_prec(binop) > op_prec {
            rhs = parse_binop(tokens, rhs, op_prec + 1);
            peek_tok = tokens.front().cloned();
        }
        lhs = AstNode::BinOp { 
            lhs: Box::new(lhs), 
            op,
            rhs: Box::new(rhs) 
        };
    }
    lhs
}

// TODO : add also parse statement

fn parse_expr(tokens : &mut VecDeque<Token>) -> AstNode {
    let lhs = parse_primary(tokens);
    parse_binop(tokens, lhs, 0)
}

pub(crate) fn parse(mut tokens : VecDeque<Token>) -> AstNode {
    parse_expr(&mut tokens)
}