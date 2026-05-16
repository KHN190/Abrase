use crate::ast::Span;
pub use crate::ast::StringPart;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // Keywords
    Fn, Let, Const, If, Else, Match, For, While, Loop, Break, Continue,
    Return, Type, Trait, Impl, Import, Mod, Pub, Region, Handle, Resume,
    Throw, True, False, Where, In, As, SelfKW, SelfUpper, Mut, Move, Thread,
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
            "region" => Token::Region,
            "handle" => Token::Handle,
            "resume" => Token::Resume,
            "throw" => Token::Throw,
            "true" => Token::True,
            "false" => Token::False,
            "where" => Token::Where,
            "in" => Token::In,
            "as" => Token::As,
            "self" => Token::SelfKW,
            "Self" => Token::SelfUpper,
            "mut" => Token::Mut,
            "move" => Token::Move,
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
                if let Some(sign @ ('+' | '-')) = self.current_char {
                    number.push(sign);
                    self.read_char();
                }
            } else {
                break;
            }
        }

        if is_float {
            match number.parse::<f64>() {
                Ok(f) => (Token::Float(f), span),
                Err(_) => (Token::Illegal(format!("float literal cannot be parsed: {}", number)), span),
            }
        } else {
            match number.parse::<i64>() {
                Ok(n) => (Token::Int(n), span),
                Err(_) => (Token::Illegal(format!("integer literal out of range: {}", number)), span),
            }
        }
    }

    fn read_escape(&mut self) -> Result<char, String> {
        self.read_char(); // skip '\'
        match self.current_char {
            Some('n')  => { self.read_char(); Ok('\n') }
            Some('t')  => { self.read_char(); Ok('\t') }
            Some('r')  => { self.read_char(); Ok('\r') }
            Some('\\') => { self.read_char(); Ok('\\') }
            Some('"')  => { self.read_char(); Ok('"')  }
            Some('\'') => { self.read_char(); Ok('\'') }
            Some('0')  => { self.read_char(); Ok('\0') }
            Some('u')  => {
                self.read_char(); // skip 'u'
                if self.current_char != Some('{') {
                    return Err("invalid unicode escape: expected '{' after \\u".into());
                }
                self.read_char(); // skip '{'
                let mut hex = String::new();
                let mut closed = false;
                while let Some(c) = self.current_char {
                    if c == '}' { self.read_char(); closed = true; break; }
                    hex.push(c);
                    self.read_char();
                }
                if !closed { return Err("unterminated unicode escape: missing '}'".into()); }
                let code = u32::from_str_radix(&hex, 16)
                    .map_err(|_| format!("invalid unicode escape: \\u{{{}}} is not valid hex", hex))?;
                char::from_u32(code)
                    .ok_or_else(|| format!("invalid unicode codepoint: U+{:04X}", code))
            }
            Some(c) => Err(format!("unknown escape sequence: \\{}", c)),
            None => Err("unterminated escape sequence at end of input".into()),
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
                '\\' => match self.read_escape() {
                    Ok(ch) => literal.push(ch),
                    Err(msg) => return (Token::Illegal(msg), span),
                },
                '{' => {
                    self.read_char(); // skip '{'
                    if !literal.is_empty() {
                        parts.push(StringPart::Literal(std::mem::take(&mut literal)));
                    }
                    let mut path: Vec<String> = Vec::new();
                    let mut seg = String::new();
                    loop {
                        match self.current_char {
                            Some('}') => {
                                self.read_char();
                                if !seg.is_empty() { path.push(seg); }
                                break;
                            }
                            Some('.') => {
                                path.push(std::mem::take(&mut seg));
                                self.read_char();
                            }
                            Some(c) if c.is_alphanumeric() || c == '_' => {
                                seg.push(c);
                                self.read_char();
                            }
                            Some(c) => {
                                return (Token::Illegal(format!(
                                    "string interpolation only supports simple paths \
                                     (e.g. {{x}} or {{a.b}}); unexpected '{}'", c
                                )), span);
                            }
                            None => {
                                return (Token::Illegal(
                                    "unterminated string interpolation".into()
                                ), span);
                            }
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
            // `has_interp` is set the only place an Interp variant is pushed,
            // so when false every part is a Literal — fall back to Illegal if
            // a future StringPart variant ever slips through.
            let mut s = String::new();
            for p in parts {
                match p {
                    StringPart::Literal(lit) => s.push_str(&lit),
                    other => return (Token::Illegal(format!(
                        "internal: non-literal StringPart in plain-string path: {:?}", other
                    )), span),
                }
            }
            return (Token::String(s), span);
        }

        (Token::StringInterp(parts), span)
    }

    fn read_char_literal(&mut self, span: Span) -> (Token, Span) {
        self.read_char(); // skip opening '
        let c = match self.current_char {
            Some('\\') => match self.read_escape() {
                Ok(ch) => ch,
                Err(msg) => return (Token::Illegal(msg), span),
            },
            Some(c) => { self.read_char(); c }
            None => return (Token::Illegal("unterminated char literal".into()), span),
        };
        if self.current_char == Some('\'') {
            self.read_char();
        } else {
            return (Token::Illegal("char literal must be a single character".into()), span);
        }
        (Token::Char(c), span)
    }
}