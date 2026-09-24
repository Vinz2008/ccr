use std::{collections::VecDeque, iter::Peekable, str::Chars};

use arrayvec::ArrayString;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum BinOp {
    Plus,
    Minus,
    Mult,
    Div,
}

#[derive(Debug, Clone)]
pub(crate) enum Token {
    Number(u128),
    BinOp(BinOp),
    Return,
    Identifier(String), // TODO : replace by identifier using a string interner (use FxHashMap, https://github.com/Vinz2008/rustaml/blob/main/src/string_intern.rs or https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html)
}

fn lex_nb(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>){
    let mut numbers = ArrayString::<19>::new();
    let mut c = chars.next();
    while let Some('0'..='9') = c {
        numbers.try_push(c.unwrap()).expect("too big of a number in int lex");
        c = chars.next();
    }

    // TODO : optimize this by transforming to a smaller number like i64, i32, etc, if small (will it really optimize it ? test it)
    let nb = numbers.as_str().parse::<u128>().unwrap();
    tokens.push_back(Token::Number(nb));
}

fn lex_op(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>){
    let binop = match chars.next().unwrap() {
        '+' => BinOp::Plus,
        '-' => BinOp::Minus,
        '*' => BinOp::Mult,
        '/' => BinOp::Div,
        c => panic!("unknown op {}", c),
    };
    tokens.push_back(Token::BinOp(binop));
}

fn lex_identifier(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>){
    let mut identifier = String::new();
    let mut c = chars.next();
    while let Some('a'..='z' | 'A'..='Z' | '_' | '0'..='9') = c {
        identifier.push(c.unwrap());
        c = chars.next();
    }
    let tok = match identifier.as_str() {
        "return" => Token::Return,
        _ => Token::Identifier(identifier),
    };
    tokens.push_back(tok);
}


pub(crate) fn lex(s : &str) -> VecDeque<Token> {
    let mut chars = s.chars().peekable();
    let mut tokens = VecDeque::new();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        match c {
            '0'..='9' => lex_nb(&mut chars, &mut tokens),
            '+' | '-' | '*' | '/' => lex_op(&mut chars, &mut tokens),
            'a'..='z' | 'A'..='Z' | '_' => lex_identifier(&mut chars, &mut tokens),
            _ => panic!("Unknown token '{}'", c),
        }
    }
    tokens
}