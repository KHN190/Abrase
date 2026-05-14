use crate::ast::Span;
pub use crate::ast::StringPart;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // Keywords
    Fn, Let, Const, If, Else, Match, For, While, Loop, Break, Continue,
    Return, Type, Trait, Impl, Import, Mod, Pub, Scope, Region, Handle,
    Throw, True, False, Where, Async, Await, In, As, SelfKW, SelfUpper, Mut, Thread,
    Effect, Underscore,

    // Identifiers and Literals
    Ident(String),
    Int(i64),
    Float(f64),
    String(String),
    StringInterp(Vec<StringPart>),
    Char(char),

    // Operators
    Assign, Plus, Minus, Asterisk, Slash, Percent,
    Eq, NotEq, Lt, Gt, Lte, Gte,
    And, Or, Bang,
    PlusAssign, MinusAssign, MulAssign, DivAssign, ModAssign,
    Range, RangeInclusive, Arrow, FatArrow,

    // Punctuation
    Comma, Colon, Semicolon, Dot, Question,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Ampersand, Pipe, At,

    Eof,
    Illegal(String),
}

pub struct Lexer<'a> {
    input: std::str::Chars<'a>,
    current_char: Option<char>,
    peek_char: Option<char>,
    pub line: usize,
    pub col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut chars = input.chars();
        let current = chars.next();
        let peek = chars.next();
        Self { input: chars, current_char: current, peek_char: peek, line: 1, col: 1 }
    }

    fn read_char(&mut self) {
        if self.current_char == Some('\n') {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        self.current_char = self.peek_char;
        self.peek_char = self.input.next();
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current_char {
            if c.is_whitespace() { self.read_char(); } else { break; }
        }
    }

    fn skip_comment(&mut self) {
        while let Some(c) = self.current_char {
            if c == '\n' { break; }
            self.read_char();
        }
    }

    fn read_identifier(&mut self) -> Token {
        let mut ident = String::new();
        while let Some(c) = self.current_char {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.read_char();
            } else {
                break;
            }
        }

        match ident.as_str() {
            "fn" => Token::Fn,
            "let" => Token::Let,
            "const" => Token::Const,
            "if" => Token::If,
            "else" => Token::Else,
            "match" => Token::Match,
            "for" => Token::For,
            "while" => Token::While,
            "loop" => Token::Loop,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "return" => Token::Return,
            "type" => Token::Type,
            "trait" => Token::Trait,
            "impl" => Token::Impl,
            "import" => Token::Import,
            "mod" => Token::Mod,
            "pub" => Token::Pub,
            "scope" => Token::Scope,
            "region" => Token::Region,
            "handle" => Token::Handle,
            "throw" => Token::Throw,
            "true" => Token::True,
            "false" => Token::False,
            "where" => Token::Where,
            "async" => Token::Async,
            "await" => Token::Await,
            "in" => Token::In,
            "as" => Token::As,
            "self" => Token::SelfKW,
            "Self" => Token::SelfUpper,
            "mut" => Token::Mut,
            "thread" => Token::Thread,
            "effect" => Token::Effect,
            "_" => Token::Underscore,
            _ => Token::Ident(ident),
        }
    }

    pub fn next_token(&mut self) -> (Token, Span) {
        self.skip_whitespace();
        
        while self.current_char == Some('/') && self.peek_char == Some('/') {
            self.skip_comment();
            self.skip_whitespace();
        }

        let start_span = Span::new(self.line, self.col);

        let token = match self.current_char {
            Some('=') => {
                if self.peek_char == Some('=') { self.read_char(); self.read_char(); Token::Eq }
                else if self.peek_char == Some('>') { self.read_char(); self.read_char(); Token::FatArrow }
                else { self.read_char(); Token::Assign }
            }
            Some('+') => {
                if self.peek_char == Some('=') { self.read_char(); self.read_char(); Token::PlusAssign }
                else { self.read_char(); Token::Plus }
            }
            Some('-') => {
                if self.peek_char == Some('=') { self.read_char(); self.read_char(); Token::MinusAssign }
                else if self.peek_char == Some('>') { self.read_char(); self.read_char(); Token::Arrow }
                else { self.read_char(); Token::Minus }
            }
            Some('*') => {
                if self.peek_char == Some('=') { self.read_char(); self.read_char(); Token::MulAssign }
                else { self.read_char(); Token::Asterisk }
            }
            Some('/') => {
                if self.peek_char == Some('=') { self.read_char(); self.read_char(); Token::DivAssign }
                else { self.read_char(); Token::Slash }
            }
            Some('%') => {
                if self.peek_char == Some('=') { self.read_char(); self.read_char(); Token::ModAssign }
                else { self.read_char(); Token::Percent }
            }
            Some('!') => {
                if self.peek_char == Some('=') { self.read_char(); self.read_char(); Token::NotEq }
                else { self.read_char(); Token::Bang }
            }
            Some('<') => {
                if self.peek_char == Some('=') { self.read_char(); self.read_char(); Token::Lte }
                else { self.read_char(); Token::Lt }
            }
            Some('>') => {
                if self.peek_char == Some('=') { self.read_char(); self.read_char(); Token::Gte }
                else { self.read_char(); Token::Gt }
            }
            Some('&') => {
                if self.peek_char == Some('&') { self.read_char(); self.read_char(); Token::And }
                else { self.read_char(); Token::Ampersand }
            }
            Some('|') => {
                if self.peek_char == Some('|') { self.read_char(); self.read_char(); Token::Or }
                else { self.read_char(); Token::Pipe }
            }
            Some('.') => {
                if self.peek_char == Some('.') {
                    self.read_char(); 
                    if self.peek_char == Some('=') { self.read_char(); self.read_char(); Token::RangeInclusive }
                    else { self.read_char(); Token::Range }
                } else { self.read_char(); Token::Dot }
            }
            Some(';') => { self.read_char(); Token::Semicolon }
            Some(':') => { self.read_char(); Token::Colon }
            Some(',') => { self.read_char(); Token::Comma }
            Some('?') => { self.read_char(); Token::Question }
            Some('(') => { self.read_char(); Token::LParen }
            Some(')') => { self.read_char(); Token::RParen }
            Some('{') => { self.read_char(); Token::LBrace }
            Some('}') => { self.read_char(); Token::RBrace }
            Some('[') => { self.read_char(); Token::LBracket }
            Some(']') => { self.read_char(); Token::RBracket }
            Some('@') => { self.read_char(); Token::At }
            Some('"') => return self.read_string(start_span),
            Some('\'') => return self.read_char_literal(start_span),
            Some(c) if c.is_alphabetic() || c == '_' => {
                let token = self.read_identifier();
                return (token, start_span);
            }
            Some(c) if c.is_ascii_digit() => {
                return self.read_number(start_span);
            }
            Some(c) => {
                self.read_char();
                Token::Illegal(c.to_string())
            }
            None => Token::Eof,
        };
        (token, start_span)
    }

    fn read_number(&mut self, span: Span) -> (Token, Span) {
        let mut number = String::new();
        let mut is_float = false;

        while let Some(c) = self.current_char {
            if c.is_ascii_digit() {
                number.push(c);
                self.read_char();
            } else if c == '.' && !is_float && self.peek_char != Some('.')
                && matches!(self.peek_char, Some(d) if d.is_ascii_digit())
            {
                is_float = true;
                number.push(c);
                self.read_char();
            } else if (c == 'e' || c == 'E') && !number.is_empty() {
                is_float = true;
                number.push(c);
                self.read_char();
                if matches!(self.current_char, Some('+') | Some('-')) {
                    number.push(self.current_char.unwrap());
                    self.read_char();
                }
            } else {
                break;
            }
        }

        if is_float {
            (Token::Float(number.parse().unwrap_or(0.0)), span)
        } else {
            (Token::Int(number.parse().unwrap_or(0)), span)
        }
    }

    /// Reads a `\` escape sequence, advancing past all consumed characters.
    /// Assumes current_char is `\` on entry.
    fn read_escape(&mut self) -> char {
        self.read_char(); // skip '\'
        match self.current_char {
            Some('n')  => { self.read_char(); '\n' }
            Some('t')  => { self.read_char(); '\t' }
            Some('r')  => { self.read_char(); '\r' }
            Some('\\') => { self.read_char(); '\\' }
            Some('"')  => { self.read_char(); '"'  }
            Some('\'') => { self.read_char(); '\'' }
            Some('0')  => { self.read_char(); '\0' }
            Some('u')  => {
                self.read_char(); // skip 'u'
                if self.current_char == Some('{') {
                    self.read_char(); // skip '{'
                    let mut hex = String::new();
                    while let Some(c) = self.current_char {
                        if c == '}' { self.read_char(); break; }
                        hex.push(c);
                        self.read_char();
                    }
                    let codepoint = u32::from_str_radix(&hex, 16).unwrap_or(0);
                    char::from_u32(codepoint).unwrap_or('\0')
                } else {
                    '\0'
                }
            }
            Some(c) => { self.read_char(); c } // unknown escape: keep literal char
            None => '\0',
        }
    }

    fn read_string(&mut self, span: Span) -> (Token, Span) {
        self.read_char(); // skip opening "
        let mut parts: Vec<StringPart> = Vec::new();
        let mut literal = String::new();
        let mut has_interp = false;

        while let Some(c) = self.current_char {
            match c {
                '"' => { self.read_char(); break; }
                '\\' => literal.push(self.read_escape()),
                '{' => {
                    self.read_char(); // skip '{'
                    if !literal.is_empty() {
                        parts.push(StringPart::Literal(std::mem::take(&mut literal)));
                    }
                    let mut path: Vec<String> = Vec::new();
                    let mut seg = String::new();
                    while let Some(ic) = self.current_char {
                        match ic {
                            '}' => {
                                self.read_char();
                                if !seg.is_empty() { path.push(seg); }
                                break;
                            }
                            '.' => {
                                path.push(std::mem::take(&mut seg));
                                self.read_char();
                            }
                            c if c.is_alphanumeric() || c == '_' => {
                                seg.push(c);
                                self.read_char();
                            }
                            _ => { self.read_char(); break; } // malformed interpolation
                        }
                    }
                    if !path.is_empty() {
                        has_interp = true;
                        parts.push(StringPart::Interp(path));
                    }
                }
                _ => { literal.push(c); self.read_char(); }
            }
        }

        if !literal.is_empty() {
            parts.push(StringPart::Literal(literal));
        }

        if !has_interp {
            let s = parts.into_iter().map(|p| match p {
                StringPart::Literal(s) => s,
                StringPart::Interp(_) => unreachable!(),
            }).collect();
            return (Token::String(s), span);
        }

        (Token::StringInterp(parts), span)
    }

    fn read_char_literal(&mut self, span: Span) -> (Token, Span) {
        self.read_char(); // skip opening '
        let c = match self.current_char {
            Some('\\') => self.read_escape(),
            Some(c) => { self.read_char(); c }
            None => '\0',
        };
        if self.current_char == Some('\'') { self.read_char(); }
        (Token::Char(c), span)
    }
}