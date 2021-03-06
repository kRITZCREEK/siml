use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
    Lambda,
    Equals,
    Dot,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LAngle,
    RAngle,
    Colon,
    DoubleColon,
    Semi,
    Comma,
    Arrow,
    FatArrow,
    Forall,
    Match,
    Let,
    LetRec,
    In,
    Type,
    Ident(String),
    UpperIdent(String),
    IntLiteral(i32),
    BooleanLiteral(bool),
}

pub struct Lexer<'input> {
    input: Peekable<Chars<'input>>,
    pos: u32,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Lexer<'input> {
        let mut lexer = Lexer {
            input: input.chars().peekable(),
            pos: 0,
        };
        lexer.consume_whitespace();
        lexer
    }

    fn next_char(&mut self) -> Option<char> {
        self.pos += 1;
        self.input.next()
    }

    fn peek(&mut self) -> Option<char> {
        self.input.peek().cloned()
    }

    fn consume_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.next_char();
            } else {
                break;
            }
        }
    }
}

fn is_ident_start(c: char) -> bool {
    match c {
        'a'..='z' | 'A'..='Z' => true,
        _ => false,
    }
}

fn is_ident_member(c: char) -> bool {
    match c {
        'a'..='z' => true,
        'A'..='Z' => true,
        '0'..='9' => true,
        '_' => true,
        _ => false,
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        let token = match self.next_char() {
            Some('\\') => Some(Token::Lambda),
            Some('.') => Some(Token::Dot),
            Some('(') => Some(Token::LParen),
            Some(')') => Some(Token::RParen),
            Some('{') => Some(Token::LBrace),
            Some('<') => Some(Token::LAngle),
            Some('>') => Some(Token::RAngle),
            Some('}') => Some(Token::RBrace),
            Some(':') => {
                if self.peek() == Some(':') {
                    self.next();
                    Some(Token::DoubleColon)
                } else {
                    Some(Token::Colon)
                }
            }
            Some(';') => Some(Token::Semi),
            Some(',') => Some(Token::Comma),
            Some('=') => {
                if self.peek() == Some('>') {
                    self.next();
                    Some(Token::FatArrow)
                } else {
                    Some(Token::Equals)
                }
            }
            Some('-') => {
                if self.peek() == Some('>') {
                    self.next();
                    Some(Token::Arrow)
                } else {
                    panic!("Failed to parse an arrow.")
                }
            }
            Some(c) if c.is_digit(10) => {
                let mut res = c.to_string();
                while let Some(c) = self.peek() {
                    if c.is_digit(10) {
                        res.push(self.next_char().unwrap())
                    } else {
                        break;
                    }
                }
                Some(Token::IntLiteral(res.parse::<i32>().unwrap()))
            }
            Some(c) if is_ident_start(c) => {
                let mut res = c.to_string();
                while let Some(c) = self.peek() {
                    if is_ident_member(c) {
                        res.push(self.next_char().unwrap())
                    } else {
                        break;
                    }
                }
                match res.as_str() {
                    "true" => Some(Token::BooleanLiteral(true)),
                    "false" => Some(Token::BooleanLiteral(false)),
                    "forall" => Some(Token::Forall),
                    "let" => Some(Token::Let),
                    "letrec" => Some(Token::LetRec),
                    "in" => Some(Token::In),
                    "match" => Some(Token::Match),
                    "type" => Some(Token::Type),
                    _ => {
                        if res.chars().next().unwrap().is_uppercase() {
                            Some(Token::UpperIdent(res))
                        } else {
                            Some(Token::Ident(res))
                        }
                    }
                }
            }
            _ => None,
        };
        self.consume_whitespace();
        debug!("Token: {:?}", &token);
        token
    }
}
