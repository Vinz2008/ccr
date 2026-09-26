use std::{collections::VecDeque, iter::Peekable, str::Chars};

use arrayvec::ArrayString;
use enum_tag::EnumTag;

use crate::types::Type;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum BinOp {
    Plus,
    Minus,
    Mult,
    Div,
    Cmp, // ==
    Equal, // =
}

// TODO : for the equal operator, implement the right associativity of operators

#[derive(Debug, Clone, EnumTag)]
pub(crate) enum Token {
    Number(u128),
    BinOp(BinOp),
    LeftParen, // ( 
    RightParen, // )
    LeftBrace, // {
    RightBrace, // }
    SemiColon, // ;
    Colon, // ,
    Return,
    If,
    Else,
    Type(Type),
    Identifier(String), // TODO : replace by identifier using a string interner (use FxHashMap, https://github.com/Vinz2008/rustaml/blob/main/src/string_intern.rs or https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html)
}

pub(crate) type TokenTag = <Token as EnumTag>::Tag;

fn lex_nb(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>){
    let mut numbers = ArrayString::<19>::new();
    let mut c = chars.peek().copied();
    while let Some('0'..='9') = c {
        chars.next();
        numbers.try_push(c.unwrap()).expect("too big of a number in int lex");
        c = chars.peek().copied();
    }

    // TODO : optimize this by transforming to a smaller number like i64, i32, etc, if small (will it really optimize it ? test it)
    let nb = numbers.as_str().parse::<u128>().unwrap();
    tokens.push_back(Token::Number(nb));
}

const BINOP_CHARS : &[char] = &[
    '+', '-', '*', '/', '=',
];

const MAX_OP_LEN : usize = 2;

fn handle_comment(chars : &mut Peekable<Chars<'_>>){
    while let Some(c) = chars.peek() && *c != '\n' {
        chars.next().unwrap();
    }
    if let Some(c) = chars.peek() && *c == '\n' {
        chars.next().unwrap();
    }
}

fn lex_op(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>){
    let mut binop : ArrayString<MAX_OP_LEN> = ArrayString::new();
    while let Some(&c) = chars.peek() && BINOP_CHARS.contains(&c){
        chars.next().unwrap();
        binop.try_push(c).expect("too long operator");
    }
    let binop = match binop.as_ref() {
        "+" => BinOp::Plus,
        "-" => BinOp::Minus,
        "*" => BinOp::Mult,
        "/" => BinOp::Div,
        "==" => BinOp::Cmp,
        "=" => BinOp::Equal,
        "//" => {
            handle_comment(chars);
            return;
        }
        op => panic!("unknown op {}", op),
    };
    
    tokens.push_back(Token::BinOp(binop));
}

fn lex_type(ident : &str) -> Type {
    match ident {
        "char" => Type::Char,
        "short" => Type::Short,
        "int" => Type::Int,
        "long" => Type::Long,
        _ => panic!("wrong type"),
    }
}

fn lex_identifier(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>){
    let mut identifier = String::new();
    let mut c = chars.peek().copied();
    while let Some('a'..='z' | 'A'..='Z' | '_' | '0'..='9') = c {
        chars.next();
        identifier.push(c.unwrap());
        c = chars.peek().copied();
    }
    let tok = match identifier.as_str() {
        "char" | "short" | "int" | "long"  => Token::Type(lex_type(identifier.as_str())), // TODO : add more types
        "return" => Token::Return,
        "if" => Token::If,
        "else" => Token::Else,
        _ => Token::Identifier(identifier),
    };
    tokens.push_back(tok);
}

fn single_char_tok(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>, tok : Token){
    tokens.push_back(tok);
    chars.next();
}

fn lex_cpp_metadata(chars : &mut Peekable<Chars<'_>>){
    handle_comment(chars);
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
            c if BINOP_CHARS.contains(&c) => lex_op(&mut chars, &mut tokens),
            'a'..='z' | 'A'..='Z' | '_' => lex_identifier(&mut chars, &mut tokens),
            '(' => single_char_tok(&mut chars, &mut tokens, Token::LeftParen),
            ')' => {
                single_char_tok(&mut chars, &mut tokens, Token::RightParen);
            },
            '{' => single_char_tok(&mut chars, &mut tokens, Token::LeftBrace),
            '}' => single_char_tok(&mut chars, &mut tokens, Token::RightBrace),
            ',' => single_char_tok(&mut chars, &mut tokens, Token::Colon),
            ';' => single_char_tok(&mut chars, &mut tokens, Token::SemiColon),
            '#' => lex_cpp_metadata(&mut chars),
            _ => panic!("Unknown token '{}'", c),
        }
    }
    tokens
}