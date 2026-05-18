use crate::ast::*;
use crate::lexer::Token;
use super::core::Parser;

impl<'a> Parser<'a> {
    pub fn parse_pattern(&mut self) -> Result<Spanned<Pattern>, String> {
        let span = self.current_span;
        let mut pats = vec![self.parse_pattern_primary()?];
        while self.peek_token == Token::Pipe {
            self.next_token();
            self.next_token();
            pats.push(self.parse_pattern_primary()?);
        }
        let node = if pats.len() == 1 {
            pats.pop().unwrap().node
        } else {
            Pattern::Or(pats)
        };
        Ok(Spanned { node, span })
    }

    pub(crate) fn parse_pattern_primary(&mut self) -> Result<Spanned<Pattern>, String> {
        let span = self.current_span;
        let node = match &self.current_token {
            Token::Underscore => Pattern::Wildcard,
            // `..` rest marker inside array / tuple patterns.
            Token::Range => Pattern::Wildcard,
            Token::Ampersand => {
                self.next_token();
                let pat = self.parse_pattern_primary()?;
                Pattern::Ref(Box::new(pat))
            }
            Token::Int(v) => {
                let start = Literal::Int(*v);
                if self.peek_token == Token::Range {
                    self.next_token();
                    self.next_token();
                    let end = if let Token::Int(n) = self.current_token { Some(Literal::Int(n)) } else { None };
                    Pattern::Range { start: Some(start), end, inclusive: false }
                } else if self.peek_token == Token::RangeInclusive {
                    self.next_token();
                    self.next_token();
                    let end = if let Token::Int(n) = self.current_token { Some(Literal::Int(n)) } else { None };
                    Pattern::Range { start: Some(start), end, inclusive: true }
                } else {
                    Pattern::Literal(start)
                }
            }
            Token::Float(v) => Pattern::Literal(Literal::Float(*v)),
            Token::True => Pattern::Literal(Literal::Bool(true)),
            Token::False => Pattern::Literal(Literal::Bool(false)),
            Token::String(s) => Pattern::Literal(Literal::String(s.clone())),
            Token::Ident(name) => {
                let name = name.clone();
                // `Option.Some`, `a.b.c`.
                let mut path = vec![name.clone()];
                while self.peek_token == Token::Dot {
                    self.next_token();
                    self.next_token();
                    if let Token::Ident(n) = &self.current_token {
                        path.push(n.clone());
                    } else {
                        return Err("Expected identifier after '.' in pattern".into());
                    }
                }
                // `Path(...)` — tuple-payload variant pattern.
                if self.peek_token == Token::LParen {
                    self.next_token();
                    let mut args = Vec::new();
                    if self.peek_token != Token::RParen {
                        self.next_token();
                        loop {
                            args.push(self.parse_pattern()?);
                            if self.peek_token == Token::Comma {
                                self.next_token();
                                self.next_token();
                            } else { break; }
                        }
                    }
                    if !self.expect_peek(Token::RParen) {
                        return Err("Expected ')' in variant pattern".into());
                    }
                    Pattern::Variant { ty: path, args }
                }
                // `Path { field, field2: pat, .. }` — record pattern.
                else if self.peek_token == Token::LBrace {
                    self.next_token();
                    self.next_token();
                    let mut fields = Vec::new();
                    let mut rest = false;
                    while self.current_token != Token::RBrace && self.current_token != Token::Eof {
                        // `..` rest marker.
                        if self.current_token == Token::Range {
                            rest = true;
                            self.next_token();
                            break;
                        }
                        let fname = if let Token::Ident(n) = &self.current_token { n.clone() } else {
                            return Err(format!("Expected field name in record pattern, got {:?}", self.current_token));
                        };
                        let inner = if self.peek_token == Token::Colon {
                            self.next_token();
                            self.next_token();
                            Some(self.parse_pattern()?)
                        } else { None };
                        fields.push(FieldPattern { name: fname, pattern: inner });
                        if self.peek_token == Token::Comma {
                            self.next_token();
                            self.next_token();
                        } else {
                            self.next_token();
                            break;
                        }
                    }
                    if self.current_token != Token::RBrace {
                        return Err("Expected '}' in record pattern".into());
                    }
                    Pattern::Record { ty: path, fields, rest }
                }
                // Bare ident 
                else if path.len() == 1 {
                    Pattern::Bind(name)
                } else {
                    // Unit variant.
                    Pattern::Variant { ty: path, args: vec![] }
                }
            }
            Token::LParen => {
                self.next_token();
                if self.current_token == Token::RParen {
                    Pattern::Tuple(vec![])
                } else {
                    let mut pats = vec![self.parse_pattern()?];
                    while self.peek_token == Token::Comma {
                        self.next_token();
                        if self.peek_token == Token::RParen { break; }
                        self.next_token();
                        pats.push(self.parse_pattern()?);
                    }
                    if !self.expect_peek(Token::RParen) {
                        return Err("Expected ')' in pattern".into());
                    }
                    Pattern::Tuple(pats)
                }
            }
            Token::LBracket => {
                self.next_token();
                if self.current_token == Token::RBracket {
                    Pattern::Array(vec![])
                } else {
                    let mut pats = vec![self.parse_pattern()?];
                    while self.peek_token == Token::Comma {
                        self.next_token();
                        if self.peek_token == Token::RBracket { break; }
                        self.next_token();
                        pats.push(self.parse_pattern()?);
                    }
                    if !self.expect_peek(Token::RBracket) {
                        return Err("Expected ']' in pattern".into());
                    }
                    Pattern::Array(pats)
                }
            }
            _ => return Err(format!("Unexpected pattern token: {:?}", self.current_token)),
        };
        Ok(Spanned { node, span })
    }
}
