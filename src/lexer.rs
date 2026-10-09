use std::{collections::VecDeque, iter::Peekable, str::Chars};

use arrayvec::ArrayString;

use crate::types::Type;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Operator {
    Plus,
    Minus,
    Mult,
    Div,
    Rem,
    Cmp, // ==
    Equal, // =
    // TODO : add other comparisons operators
    Lower, // <
}

// operators which are only unary (++, --, etc)
#[derive(Debug, Clone, Copy)]
pub(crate) enum UnaryOp {
    PostfixPlus,
    // TODO : add more postfix operators
}

#[derive(Debug, Clone)]
pub(crate) enum Token {
    Number(u128),
    Operator(Operator),
    UnaryOp(UnaryOp),
    Char(char),
    LeftParen, // ( 
    RightParen, // )
    LeftBrace, // {
    RightBrace, // }
    SemiColon, // ;
    Colon, // ,
    Return,
    If,
    Else,
    While,
    For,
    Type(Type),
    Identifier(Box<str>), // TODO : replace by identifier using a string interner (use FxHashMap, https://github.com/Vinz2008/rustaml/blob/main/src/string_intern.rs or https://matklad.github.io/2020/03/22/fast-simple-rust-interner.html)
    String(Box<str>),
}

// TODO : add digraphs ? (<:, :>, <%, %>, etc)

fn eat_char(chars : &mut Peekable<Chars<'_>>, expected_c : char){
    let c = chars.next().unwrap(); // eat '
    if c != expected_c {
        panic!("expected char {}, got {}", expected_c, c);
    }
}

fn lex_nb(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>){
    let mut numbers = ArrayString::<19>::new();
    let mut c = chars.peek().copied();
    while let Some('0'..='9') = c {
        chars.next().unwrap();
        numbers.try_push(c.unwrap()).expect("too big of a number in int lex");
        c = chars.peek().copied();
    }

    // TODO : optimize this by transforming to a smaller number like i64, i32, etc, if small (will it really optimize it ? test it)
    let nb = numbers.as_str().parse::<u128>().unwrap();
    tokens.push_back(Token::Number(nb));
}

const BINOP_CHARS : &[char] = &[
    '+', '-', '*', '/', '=', '<'
];

const MAX_OP_LEN : usize = 2;

fn lex_single_line_comment(chars : &mut Peekable<Chars<'_>>){
    while let Some(c) = chars.peek() && *c != '\n' {
        chars.next().unwrap();
    }
    if let Some(c) = chars.peek() && *c == '\n' {
        chars.next().unwrap();
    }
}

fn lex_multi_line_comment(chars : &mut Peekable<Chars<'_>>){
    loop {
        if let Some('*') = chars.peek(){
            chars.next().unwrap();
            if let Some('/') = chars.peek(){
                break;
            }
        } else {
            chars.next().unwrap();
        }
    }
}

fn lex_op(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>){
    let mut binop : ArrayString<MAX_OP_LEN> = ArrayString::new();
    while let Some(&c) = chars.peek() && BINOP_CHARS.contains(&c){
        chars.next().unwrap();
        binop.try_push(c).expect("too long operator");
    }
    let binop = match binop.as_ref() {
        "+" => Operator::Plus,
        "-" => Operator::Minus,
        "*" => Operator::Mult,
        "/" => Operator::Div,
        "==" => Operator::Cmp,
        "=" => Operator::Equal,
        "<" => Operator::Lower,
        "++" => {
            tokens.push_back(Token::UnaryOp(UnaryOp::PostfixPlus));
            return;
        },
        "//" => {
            lex_single_line_comment(chars);
            return;
        }
        "/*" => {
            lex_multi_line_comment(chars);
            return;
        }
        op => panic!("unknown op {}", op),
    };
    
    tokens.push_back(Token::Operator(binop));
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
        chars.next().unwrap();
        identifier.push(c.unwrap());
        c = chars.peek().copied();
    }
    let tok = match identifier.as_str() {
        "char" | "short" | "int" | "long"  => Token::Type(lex_type(identifier.as_str())), // TODO : add more types
        "return" => Token::Return,
        "if" => Token::If,
        "else" => Token::Else,
        "while" => Token::While,
        "for" => Token::For,
        _ => Token::Identifier(identifier.into_boxed_str()),
    };
    tokens.push_back(tok);
}

fn single_char_tok(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>, tok : Token){
    tokens.push_back(tok);
    chars.next().unwrap();
}

fn lex_cpp_metadata(chars : &mut Peekable<Chars<'_>>){
    lex_single_line_comment(chars);
}

// TODO : for the string case, could even search for the backlash in a quicker wqy (simd ? simd in 64 bits reg ?) to find quickly if there is \, and if not to have a fast path

fn lex_char_inner(chars : &mut Peekable<Chars<'_>>) -> char {
    let mut c = chars.next().unwrap();
    if c == '\\' {
        let escape_c = chars.next().unwrap();
        // TODO : could use a big table instead (would need to make mandatory chars here ascii, but then why not make the lexing be on bytes to not have decoding overhead)
        // TODO : \x and \o
        c = match escape_c {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            'a' => '\x07',
            'b' => '\x08',
            'f' => '\x0C',
            'v' => '\x0B',
            _ => escape_c,
        };
    };
    c
}

fn lex_char_lit(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>){
    eat_char(chars, '\'');
    let c = lex_char_inner(chars);
    tokens.push_back(Token::Char(c));
    eat_char(chars, '\'');
}

fn lex_string(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>){
    eat_char(chars, '\"');
    let mut str = String::new();
    let mut c = chars.peek().copied();
    while let Some(c_) = c && c_ != '"' {
        let lexed_char = lex_char_inner(chars);
        str.push(lexed_char);
        c = chars.peek().copied();
    }
    eat_char(chars, '\"');
    tokens.push_back(Token::String(str.into_boxed_str()));
}

// TODO : add line infos to the tokens (should I make a token a struct ? should I have a separate VecDequeue for line infos ?)

pub(crate) fn lex(s : &str) -> VecDeque<Token> {
    let mut chars = s.chars().peekable();
    let mut tokens = VecDeque::new();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next().unwrap();
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
            '\'' => lex_char_lit(&mut chars, &mut tokens),
            '"' => lex_string(&mut chars, &mut tokens),
            _ => panic!("Unknown token '{}'", c),
        }
    }
    tokens
}