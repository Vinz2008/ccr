use std::{collections::VecDeque, iter::Peekable, str::Chars};

use arrayvec::ArrayString;

use crate::types::{IntegerType, Sign, Type};

#[derive(Debug)]
pub(crate) enum LexerError {
    UnexpectedEof,
    WrongChar {
        expected : char,
        got : char,
    },
    UnexpectedChar(char),
    TooLongNumber {
        nb : Box<str>,
    },
    TooLongOp {
        op : Box<str>,
    },
    UnknownOp(Box<str>),
    WrongType(Box<str>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Operator {
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
    PlusPlus, // ++
    MinusMinus, // --
}


#[derive(Debug, Clone)]
pub(crate) enum Token {
    Number(u128),
    Operator(Operator),
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

fn eat_char(chars : &mut Peekable<Chars<'_>>, expected_c : char) -> Result<(), LexerError> {
    let c = chars.next().ok_or(LexerError::UnexpectedEof)?; // eat '
    if c != expected_c {
        return Err(LexerError::WrongChar { expected: expected_c, got: c });
    }
    Ok(())
}

fn lex_nb(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>) -> Result<(), LexerError> {
    let mut numbers = ArrayString::<19>::new();
    let mut c = chars.peek().copied();
    while let Some('0'..='9') = c {
        let c_u = c.unwrap();
        chars.next().ok_or(LexerError::UnexpectedEof)?;
        numbers.try_push(c_u).map_err(|_| LexerError::TooLongNumber { nb: format!("{}{}", numbers, c_u).into_boxed_str() })?;
        c = chars.peek().copied();
    }

    // TODO : optimize this by transforming to a smaller number like i64, i32, etc, if small (will it really optimize it ? test it)
    let nb = numbers.as_str().parse::<u128>().unwrap();
    tokens.push_back(Token::Number(nb));
    Ok(())
}

const BINOP_CHARS : &[char] = &[
    '+', '-', '*', '/', '=', '<'
];

const MAX_OP_LEN : usize = 2;

fn lex_single_line_comment(chars : &mut Peekable<Chars<'_>>) -> Result<(), LexerError>{
    while let Some(c) = chars.peek() && *c != '\n' {
        chars.next().ok_or(LexerError::UnexpectedEof)?;
    }
    if let Some(c) = chars.peek() && *c == '\n' {
        chars.next().ok_or(LexerError::UnexpectedEof)?;
    }
    Ok(())
}

fn lex_multi_line_comment(chars : &mut Peekable<Chars<'_>>) -> Result<(), LexerError> {
    loop {
        if let Some('*') = chars.peek(){
            chars.next().ok_or(LexerError::UnexpectedEof)?;
            if let Some('/') = chars.peek(){
                break;
            }
        } else {
            chars.next().ok_or(LexerError::UnexpectedEof)?;
        }
    }
    Ok(())
}

fn lex_op(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>) -> Result<(), LexerError> {
    let mut binop : ArrayString<MAX_OP_LEN> = ArrayString::new();
    while let Some(&c) = chars.peek() && BINOP_CHARS.contains(&c){
        chars.next().ok_or(LexerError::UnexpectedEof)?;
        binop.try_push(c).map_err(|_| LexerError::TooLongOp { op: format!("{}{}", binop, c).into_boxed_str() })?;
    }
    let binop = match binop.as_ref() {
        "+" => Operator::Plus,
        "-" => Operator::Minus,
        "*" => Operator::Mult,
        "/" => Operator::Div,
        "==" => Operator::Cmp,
        "=" => Operator::Equal,
        "<" => Operator::Lower,
        ">" => Operator::Upper,
        "<=" => Operator::LowerEqual,
        ">=" => Operator::UpperEqual,
        "++" => Operator::PlusPlus,
        "--" => Operator::MinusMinus,
        "//" => {
            return lex_single_line_comment(chars);
        }
        "/*" => {
            return lex_multi_line_comment(chars);
        }
        op => return Err(LexerError::UnknownOp(Box::from(op))),
    };
    
    tokens.push_back(Token::Operator(binop));
    Ok(())
}

fn lex_identifier_inner(chars : &mut Peekable<Chars<'_>>) -> Result<String, LexerError> {
    let mut identifier = String::new();
    let mut c = chars.peek().copied();
    while let Some('a'..='z' | 'A'..='Z' | '_' | '0'..='9') = c {
        chars.next().ok_or(LexerError::UnexpectedEof)?;
        identifier.push(c.unwrap());
        c = chars.peek().copied();
    }
    Ok(identifier)
}

fn pass_space(chars : &mut Peekable<Chars<'_>>) -> Result<(), LexerError> {
    while let Some(c) = chars.peek().copied() && c.is_whitespace() {
        chars.next().ok_or(LexerError::UnexpectedEof)?;
    }
    Ok(())
}

// TODO : maybe refactor the lexing of the type (should I check the grammar for it ? should it be more in parsing instead ?)

fn lex_identifier_for_type(chars : &mut Peekable<Chars<'_>>) -> Result<String, LexerError> {
    pass_space(chars)?;
    lex_identifier_inner(chars)
}

// TODO : add const in the type (need to make Type a struct with a is_const bool and a type inner enum)
fn lex_type_inner(chars : &mut Peekable<Chars<'_>>, ident : &str, is_const : bool, is_unsigned : bool) -> Result<Type, LexerError> {
    let sign = if is_unsigned {
        Sign::Unsigned
    } else {
        Sign::Signed
    };
    let mut res_type = match ident {
        "char" => Type::Integer(sign, IntegerType::Char),
        "short" => Type::Integer(sign, IntegerType::Short),
        "int" => Type::Integer(sign, IntegerType::Int),
        "long" => Type::Integer(sign, IntegerType::Long),
        _ => return Err(LexerError::WrongType(Box::from(ident))), // TODO : also support having just unsigned for the type
    };
    pass_space(chars)?;
    while let Some('*') = chars.peek().copied(){
        res_type = Type::Ptr(Box::new(res_type));
        chars.next().ok_or(LexerError::UnexpectedEof)?;
    }
    Ok(res_type)
}

fn _lex_type(chars : &mut Peekable<Chars<'_>>, ident : &str, is_const : bool, is_unsigned : bool) -> Result<Type, LexerError> {
    match ident {
        "const" if !is_const => {
            let next_ident = lex_identifier_for_type(chars)?;
            _lex_type(chars, &next_ident, true, is_unsigned)
        },
        "unsigned" if !is_unsigned => {
            let next_ident = lex_identifier_for_type(chars)?;
            _lex_type(chars, &next_ident, is_const, true)
        }
        _ => {
            lex_type_inner(chars, ident, is_const, is_unsigned)
        }
    }
}

fn lex_type(chars : &mut Peekable<Chars<'_>>, ident : &str) -> Result<Type, LexerError> {
    _lex_type(chars, ident, false, false)
}

fn lex_identifier(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>) -> Result<(), LexerError>{
    let identifier = lex_identifier_inner(chars)?;
    let tok = match identifier.as_str() {
        "char" | "short" | "int" | "long" | "const" | "unsigned"  => Token::Type(lex_type(chars, identifier.as_str())?), // TODO : add more types
        "return" => Token::Return,
        "if" => Token::If,
        "else" => Token::Else,
        "while" => Token::While,
        "for" => Token::For,
        _ => Token::Identifier(identifier.into_boxed_str()),
    };
    tokens.push_back(tok);
    Ok(())
}

fn single_char_tok(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>, tok : Token) -> Result<(), LexerError>{
    tokens.push_back(tok);
    chars.next().ok_or(LexerError::UnexpectedEof)?;
    Ok(())
}

fn lex_cpp_metadata(chars : &mut Peekable<Chars<'_>>) -> Result<(), LexerError>{
    lex_single_line_comment(chars)
}

// TODO : for the string case, could even search for the backlash in a quicker way (simd ? simd in 64 bits reg ?) to find quickly if there is \, and if not to have a fast path

fn lex_char_inner(chars : &mut Peekable<Chars<'_>>) -> Result<char, LexerError> {
    let mut c = chars.next().ok_or(LexerError::UnexpectedEof)?;
    if c == '\\' {
        let escape_c = chars.next().ok_or(LexerError::UnexpectedEof)?;
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
    Ok(c)
}

fn lex_char_lit(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>) -> Result<(), LexerError>{
    eat_char(chars, '\'')?;
    let c = lex_char_inner(chars)?;
    tokens.push_back(Token::Char(c));
    eat_char(chars, '\'')?;
    Ok(())
}

fn lex_string(chars : &mut Peekable<Chars<'_>>, tokens : &mut VecDeque<Token>) -> Result<(), LexerError>{
    eat_char(chars, '\"')?;
    let mut str = String::new();
    let mut c = chars.peek().copied();
    while let Some(c_) = c && c_ != '"' {
        let lexed_char = lex_char_inner(chars)?;
        str.push(lexed_char);
        c = chars.peek().copied();
    }
    eat_char(chars, '\"')?;
    tokens.push_back(Token::String(str.into_boxed_str()));
    Ok(())
}

// TODO : add line infos to the tokens (should I make a token a struct ? should I have a separate VecDequeue for line infos ?)

pub(crate) fn lex(s : &str) -> Result<VecDeque<Token>, LexerError> {
    let mut chars = s.chars().peekable();
    let mut tokens = VecDeque::new();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next().ok_or(LexerError::UnexpectedEof)?;
            continue;
        }
        match c {
            '0'..='9' => lex_nb(&mut chars, &mut tokens),
            c if BINOP_CHARS.contains(&c) => lex_op(&mut chars, &mut tokens),
            'a'..='z' | 'A'..='Z' | '_' => lex_identifier(&mut chars, &mut tokens),
            '(' => single_char_tok(&mut chars, &mut tokens, Token::LeftParen),
            ')' => single_char_tok(&mut chars, &mut tokens, Token::RightParen),
            '{' => single_char_tok(&mut chars, &mut tokens, Token::LeftBrace),
            '}' => single_char_tok(&mut chars, &mut tokens, Token::RightBrace),
            ',' => single_char_tok(&mut chars, &mut tokens, Token::Colon),
            ';' => single_char_tok(&mut chars, &mut tokens, Token::SemiColon),
            '#' => lex_cpp_metadata(&mut chars),
            '\'' => lex_char_lit(&mut chars, &mut tokens),
            '"' => lex_string(&mut chars, &mut tokens),
            _ => return Err(LexerError::UnexpectedChar(c)),
        }?
    }
    Ok(tokens)
}