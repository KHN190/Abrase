use crate::ast::*;
use crate::lexer::Token;
use super::core::Parser;

impl<'a> Parser<'a> {
    pub fn parse_type(&mut self) -> Result<Type, String> {
        match self.current_token.clone() {
            Token::Ident(name) => {
                if self.peek_token == Token::ColonColon {
                    let mut path = vec![name];
                    while self.peek_token == Token::ColonColon {
                        self.next_token();
                        self.next_token();
                        match self.current_token.clone() {
                            Token::Ident(n) => path.push(n),
                            _ => return Err("Expected ident in qualified type".into()),
                        }
                    }
                    return Ok(Type::Qualified(path));
                }
                if self.peek_token == Token::Lt {
                    self.next_token();
                    let args = self.parse_type_args()?;
                    return Ok(Type::Generic { name, args });
                }
                Ok(Type::Named(name))
            }
            Token::SelfUpper => Ok(Type::Named("Self".into())),
            Token::Fn => {
                if !self.expect_peek(Token::LParen) {
                    return Err("Expected '(' after 'fn' in function type".into());
                }
                let mut params = Vec::new();
                if self.peek_token != Token::RParen {
                    self.next_token();
                    params.push(self.parse_type()?);
                    while self.peek_token == Token::Comma {
                        self.next_token();
                        if self.peek_token == Token::RParen { break; }
                        self.next_token();
                        params.push(self.parse_type()?);
                    }
                }
                if !self.expect_peek(Token::RParen) {
                    return Err("Expected ')' in function type".into());
                }
                if self.peek_token == Token::Arrow {
                    self.next_token();
                    let (effects, ret) = self.parse_fn_type_tail()?;
                    Ok(Type::Function { params, effects, ret: Box::new(ret) })
                } else {
                    Ok(Type::Function { params, effects: vec![], ret: Box::new(Type::Tuple(vec![])) })
                }
            }
            Token::Ampersand => {
                self.next_token();
                let is_mut = if self.current_token == Token::Mut {
                    self.next_token();
                    true
                } else { false };
                let inner = self.parse_type()?;
                let region = if self.peek_token == Token::In {
                    self.next_token();
                    self.next_token();
                    match self.current_token.clone() {
                        Token::Ident(r) => Some(r),
                        _ => return Err("Expected region name after 'in'".into()),
                    }
                } else { None };
                Ok(Type::Reference { is_mut, inner: Box::new(inner), region })
            }
            Token::LBracket => {
                self.next_token();
                let elem = self.parse_type()?;
                if !self.expect_peek(Token::Semicolon) {
                    return Err("Expected ';' in array type".into());
                }
                self.next_token();
                let size = match &self.current_token {
                    Token::Int(n) => *n as usize,
                    _ => return Err("Expected integer size in array type".into()),
                };
                if !self.expect_peek(Token::RBracket) {
                    return Err("Expected ']' in array type".into());
                }
                Ok(Type::Array { elem: Box::new(elem), size })
            }
            Token::LParen => {
                if self.peek_token == Token::RParen {
                    self.next_token();
                    if self.peek_token == Token::Arrow {
                        self.next_token();
                        let (effects, ret) = self.parse_fn_type_tail()?;
                        return Ok(Type::Function { params: vec![], effects, ret: Box::new(ret) });
                    }
                    return Ok(Type::Tuple(vec![]));
                }
                self.next_token();
                let mut types = vec![self.parse_type()?];
                while self.peek_token == Token::Comma {
                    self.next_token();
                    if self.peek_token == Token::RParen { break; }
                    self.next_token();
                    types.push(self.parse_type()?);
                }
                if !self.expect_peek(Token::RParen) {
                    return Err("Expected ')' in type".into());
                }
                if self.peek_token == Token::Arrow {
                    self.next_token();
                    let (effects, ret) = self.parse_fn_type_tail()?;
                    return Ok(Type::Function { params: types, effects, ret: Box::new(ret) });
                }
                Ok(Type::Tuple(types))
            }
            _ => Err(format!("Unknown type token: {:?}", self.current_token)),
        }
    }

    pub(crate) fn parse_type_args(&mut self) -> Result<Vec<Type>, String> {
        if self.peek_is_generic_close() {
            self.expect_peek_generic_close();
            return Ok(vec![]);
        }
        self.next_token();
        let mut args = vec![self.parse_type()?];
        while self.peek_token == Token::Comma {
            self.next_token();
            if self.peek_is_generic_close() { break; }
            self.next_token();
            args.push(self.parse_type()?);
        }
        if !self.expect_peek_generic_close() {
            return Err("Expected '>' in type args".into());
        }
        Ok(args)
    }

    pub(crate) fn parse_fn_type_tail(&mut self) -> Result<(Vec<EffectItem>, Type), String> {
        let effects = if self.peek_token == Token::Lt {
            self.next_token();
            self.parse_effect_set()?
        } else { vec![] };
        self.next_token();
        let ret = self.parse_type()?;
        Ok((effects, ret))
    }

    pub(crate) fn parse_effect_set(&mut self) -> Result<Vec<EffectItem>, String> {
        if self.peek_is_generic_close() {
            self.expect_peek_generic_close();
            return Ok(vec![]);
        }
        self.next_token();
        let mut effects = Vec::new();
        loop {
            let mut name = Vec::new();
            match self.current_token.clone() {
                Token::Ident(n) => name.push(n),
                _ => return Err("Expected effect name".into()),
            }
            while self.peek_token == Token::Dot {
                self.next_token();
                self.next_token();
                match self.current_token.clone() {
                    Token::Ident(n) => name.push(n),
                    _ => return Err("Expected ident in effect name".into()),
                }
            }
            let arg = if self.peek_token == Token::Lt {
                self.next_token();
                self.next_token();
                let ty = self.parse_type()?;
                if !self.expect_peek_generic_close() {
                    return Err("Expected '>' after effect arg".into());
                }
                Some(Box::new(ty))
            } else { None };
            effects.push(EffectItem { name, arg });
            if self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
            } else { break; }
        }
        if !self.expect_peek_generic_close() {
            return Err("Expected '>' after effect set".into());
        }
        Ok(effects)
    }
}
