use crate::ast::*;
use crate::lexer::Token;
use super::core::Parser;
use super::precedence::Precedence;
use super::helpers::is_block_terminated;

impl<'a> Parser<'a> {
    pub fn parse_program(&mut self) -> Vec<Decl> {
        let mut decls = Vec::new();
        while self.current_token != Token::Eof {
            let err_span = self.current_span;
            match self.parse_decl() {
                Ok(decl) => {
                    decls.push(decl);
                    if self.current_token != Token::Eof &&
                       !matches!(self.current_token, Token::Fn | Token::Type | Token::Trait | Token::Impl | Token::Const | Token::Import | Token::Effect | Token::Mod | Token::Pub) {
                        self.report_error(format!("Unexpected token after declaration: {:?}", self.current_token), self.current_span);
                        self.synchronize();
                    }
                }
                Err(msg) => {
                    self.report_error(msg, err_span);
                    self.synchronize();
                }
            }
        }
        decls
    }

    pub fn parse_decl(&mut self) -> Result<Decl, String> {
        let attrs = if self.current_token == Token::At {
            self.parse_attributes()?
        } else { vec![] };

        let mut is_pub = false;

        if self.current_token == Token::Pub {
            is_pub = true;
            self.next_token();
        }

        match self.current_token {
            Token::Fn => self.parse_fn_decl_with_attrs(is_pub, attrs),
            Token::Type => {
                self.next_token();
                if self.current_token == Token::Ident("alias".into()) {
                    self.next_token();
                    self.parse_type_alias_decl(is_pub)
                } else {
                    self.parse_type_decl_with_attrs(is_pub, attrs)
                }
            }
            Token::Trait => self.parse_trait_decl(is_pub),
            Token::Impl => self.parse_impl_decl(),
            Token::Const => self.parse_const_decl(is_pub),
            Token::Import => self.parse_import_decl(),
            Token::Effect => {
                self.next_token();
                if self.current_token == Token::Ident("alias".into()) {
                    self.next_token();
                    self.parse_effect_alias_decl(is_pub)
                } else {
                    self.parse_effect_decl(is_pub)
                }
            }
            Token::Mod => {
                self.next_token();
                let mut path = Vec::new();
                if let Token::Ident(p) = &self.current_token {
                    path.push(p.clone());
                } else {
                    return Err("Expected module name".into());
                }
                while self.peek_token == Token::Dot {
                    self.next_token();
                    self.next_token();
                    if let Token::Ident(p) = &self.current_token {
                        path.push(p.clone());
                    } else {
                        return Err("Expected ident in module path".into());
                    }
                }
                self.next_token();
                Ok(Decl::Mod(path.join(".")))
            }
            _ => Err(format!("Unexpected declaration token: {:?}", self.current_token)),
        }
    }

    fn parse_attributes(&mut self) -> Result<Vec<Attribute>, String> {
        let mut attrs = Vec::new();
        while self.current_token == Token::At {
            self.next_token();
            let name = if let Token::Ident(n) = &self.current_token { n.clone() } else {
                return Err("Expected attribute name after '@'".into());
            };
            let mut args = Vec::new();
            if self.peek_token == Token::LParen {
                self.next_token();
                if self.peek_token != Token::RParen {
                    self.next_token();
                    loop {
                        let arg = match self.current_token.clone() {
                            Token::Ident(n) => {
                                if self.peek_token == Token::Assign {
                                    self.next_token();
                                    self.next_token();
                                    let lit = self.token_to_literal()?;
                                    AttrArg::Named(n, lit)
                                } else {
                                    AttrArg::Ident(n)
                                }
                            }
                            _ => AttrArg::Lit(self.token_to_literal()?),
                        };
                        args.push(arg);
                        if self.peek_token == Token::Comma {
                            self.next_token();
                            self.next_token();
                        } else { break; }
                    }
                }
                if !self.expect_peek(Token::RParen) {
                    return Err("Expected ')' in attribute".into());
                }
            }
            attrs.push(Attribute { name, args });
            self.next_token();
        }
        Ok(attrs)
    }

    fn token_to_literal(&self) -> Result<Literal, String> {
        match &self.current_token {
            Token::Int(v) => Ok(Literal::Int(*v)),
            Token::Float(v) => Ok(Literal::Float(*v)),
            Token::String(s) => Ok(Literal::String(s.clone())),
            Token::True => Ok(Literal::Bool(true)),
            Token::False => Ok(Literal::Bool(false)),
            other => Err(format!("Expected literal, got {:?}", other)),
        }
    }

    fn parse_type_decl_with_attrs(
        &mut self,
        is_pub: bool,
        attrs: Vec<Attribute>,
    ) -> Result<Decl, String> {
        let name = if let Token::Ident(n) = &self.current_token { n.clone() } else { return Err("Expected type name".into()); };
        let generics = if self.peek_token == Token::Lt {
            self.next_token();
            self.parse_generic_params()?
        } else { vec![] };

        if !self.expect_peek(Token::Assign) {
            return Err("Expected '=' in type decl".into());
        }
        self.next_token();

        let body = if self.current_token == Token::LBrace {
            self.next_token();
            let mut fields = Vec::new();
            while self.current_token != Token::RBrace && self.current_token != Token::Eof {
                if let Token::Ident(fname) = &self.current_token {
                    let fname = fname.clone();
                    if !self.expect_peek(Token::Colon) {
                        return Err("Expected ':' in record field".into());
                    }
                    self.next_token();
                    let ftype = self.parse_type()?;
                    fields.push(RecordField { is_pub: false, name: fname, ty: ftype });
                    if self.peek_token == Token::Comma {
                        self.next_token();
                        self.next_token();
                    } else { break; }
                } else { break; }
            }
            if !self.expect_peek(Token::RBrace) {
                return Err("Expected '}' in record type".into());
            }
            self.next_token();
            TypeBody::Record(fields)
        } else if matches!(self.current_token, Token::Ident(_)) {
            let mut cases = Vec::new();
            loop {
                let case_name = if let Token::Ident(n) = &self.current_token { n.clone() } else {
                    return Err("Expected variant constructor name".into());
                };
                let case = if self.peek_token == Token::LParen {
                    self.next_token();
                    self.next_token();
                    let mut tys = Vec::new();
                    while self.current_token != Token::RParen && self.current_token != Token::Eof {
                        tys.push(self.parse_type()?);
                        if self.peek_token == Token::Comma {
                            self.next_token();
                            self.next_token();
                        } else { break; }
                    }
                    if !self.expect_peek(Token::RParen) {
                        return Err("Expected ')' in variant".into());
                    }
                    VariantCase::Tuple(case_name, tys)
                } else if self.peek_token == Token::LBrace {
                    self.next_token();
                    self.next_token();
                    let mut fs = Vec::new();
                    while self.current_token != Token::RBrace && self.current_token != Token::Eof {
                        if let Token::Ident(fname) = &self.current_token {
                            let fname = fname.clone();
                            if !self.expect_peek(Token::Colon) {
                                return Err("Expected ':' in variant field".into());
                            }
                            self.next_token();
                            let ftype = self.parse_type()?;
                            fs.push(RecordField { is_pub: false, name: fname, ty: ftype });
                            if self.peek_token == Token::Comma {
                                self.next_token();
                                self.next_token();
                            } else { break; }
                        } else { break; }
                    }
                    if !self.expect_peek(Token::RBrace) {
                        return Err("Expected '}' in variant record".into());
                    }
                    VariantCase::Record(case_name, fs)
                } else {
                    VariantCase::Unit(case_name)
                };
                cases.push(case);
                if self.peek_token == Token::Pipe {
                    self.next_token();
                    self.next_token();
                } else { break; }
            }
            self.next_token();
            TypeBody::Variant(cases)
        } else {
            return Err("Expected type body".into());
        };

        Ok(Decl::Type { attrs, is_pub, ownership: None, name, generics, body })
    }

    fn parse_trait_decl(&mut self, is_pub: bool) -> Result<Decl, String> {
        self.next_token();
        let name = if let Token::Ident(n) = &self.current_token { n.clone() } else { return Err("Expected trait name".into()); };

        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in trait".into());
        }

        let mut items = Vec::new();
        self.next_token();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            if self.current_token == Token::Fn {
                let fn_decl = self.parse_fn_decl(false)?;
                if let Decl::Fn(f) = fn_decl {
                    items.push(TraitItem::Default(f));
                }
            }
            if self.current_token != Token::RBrace {
                self.next_token();
            }
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Ok(Decl::Trait { is_pub, name, generics: vec![], where_clause: vec![], items })
    }

    fn parse_impl_decl(&mut self) -> Result<Decl, String> {
        self.next_token();
        let head_type = self.parse_type()?;

        let (trait_name, for_type) = if self.peek_token == Token::For {
            self.next_token();
            self.next_token();
            let target = self.parse_type()?;
            let trait_name = match head_type {
                Type::Named(n) => Some(vec![n]),
                Type::Qualified(parts) => Some(parts),
                _ => None,
            };
            (trait_name, target)
        } else {
            (None, head_type)
        };

        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in impl".into());
        }

        let mut methods = Vec::new();
        self.next_token();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            if self.current_token == Token::Fn {
                let fn_decl = self.parse_fn_decl(false)?;
                if let Decl::Fn(f) = fn_decl {
                    methods.push(f);
                }
            }
            if self.current_token != Token::RBrace {
                self.next_token();
            }
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Ok(Decl::Impl { generics: vec![], trait_name, for_type, where_clause: vec![], methods })
    }

    fn parse_const_decl(&mut self, is_pub: bool) -> Result<Decl, String> {
        self.next_token();
        let is_fn = self.current_token == Token::Fn;
        if is_fn {
            self.next_token();
        }
        let name = if let Token::Ident(n) = &self.current_token { n.clone() } else { return Err("Expected const name".into()); };

        let generics = if self.peek_token == Token::Lt {
            self.next_token();
            self.parse_generic_params()?
        } else { vec![] };

        let params = if self.peek_token == Token::LParen {
            self.next_token();
            self.parse_params()?
        } else { vec![] };

        if !self.expect_peek(Token::Colon) {
            return Err("Expected ':' in const".into());
        }
        self.next_token();
        let ty = self.parse_type()?;

        if !self.expect_peek(Token::Assign) {
            return Err("Expected '=' in const".into());
        }
        self.next_token();
        let value = self.parse_expr(Precedence::Lowest);

        let was_block = is_block_terminated(&value.node);
        if !was_block {
            if self.peek_token == Token::Semicolon {
                self.next_token();
            }
            self.next_token();
        } else if self.current_token == Token::Semicolon {
            self.next_token();
        }

        Ok(Decl::Const { is_pub, is_fn, name, generics, params, ty, value })
    }

    fn parse_type_alias_decl(&mut self, is_pub: bool) -> Result<Decl, String> {
        let name = if let Token::Ident(n) = &self.current_token { n.clone() } else { return Err("Expected type alias name".into()); };
        let generics = if self.peek_token == Token::Lt {
            self.next_token();
            self.parse_generic_params()?
        } else { vec![] };
        if !self.expect_peek(Token::Assign) {
            return Err("Expected '=' in type alias".into());
        }
        self.next_token();
        let ty = self.parse_type()?;
        if self.peek_token == Token::Semicolon {
            self.next_token();
        }
        self.next_token();
        Ok(Decl::TypeAlias { is_pub, name, generics, ty })
    }

    fn parse_effect_alias_decl(&mut self, is_pub: bool) -> Result<Decl, String> {
        let name = if let Token::Ident(n) = &self.current_token { n.clone() } else { return Err("Expected effect alias name".into()); };
        if !self.expect_peek(Token::Assign) {
            return Err("Expected '=' in effect alias".into());
        }
        self.next_token();
        let effects = if self.current_token == Token::Lt {
            self.parse_effect_set()?
        } else {
            return Err("Expected effect set in effect alias".into());
        };
        if self.peek_token == Token::Semicolon {
            self.next_token();
        }
        self.next_token();
        Ok(Decl::EffectAlias { is_pub, name, effects })
    }

    fn parse_effect_decl(&mut self, is_pub: bool) -> Result<Decl, String> {
        let name = if let Token::Ident(n) = &self.current_token { n.clone() } else { return Err("Expected effect name".into()); };
        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in effect decl".into());
        }
        let mut ops = Vec::new();
        self.next_token();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            if self.current_token == Token::Fn {
                let sig = self.parse_fn_signature()?;
                ops.push(sig);
            }
            if self.current_token != Token::RBrace {
                self.next_token();
            }
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Ok(Decl::Effect { is_pub, name, ops })
    }

    fn parse_generic_params(&mut self) -> Result<Vec<GenericParam>, String> {
        if self.peek_token == Token::Gt {
            self.next_token();
            return Ok(vec![]);
        }
        self.next_token();
        let mut params = Vec::new();
        loop {
            if let Token::Ident(n) = &self.current_token {
                params.push(GenericParam { name: n.clone() });
            } else {
                return Err("Expected generic param name".into());
            }
            if self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
            } else { break; }
        }
        if !self.expect_peek(Token::Gt) {
            return Err("Expected '>' in generic params".into());
        }
        Ok(params)
    }

    fn parse_fn_signature(&mut self) -> Result<FnSignature, String> {
        self.next_token();
        let name = if let Token::Ident(n) = &self.current_token { n.clone() } else { return Err("Expected fn name in signature".into()); };
        if !self.expect_peek(Token::LParen) {
            return Err("Expected '(' in fn signature".into());
        }
        let params = self.parse_params()?;
        let return_type = if self.peek_token == Token::Arrow {
            self.next_token();
            self.next_token();
            Some(self.parse_type()?)
        } else { None };
        Ok(FnSignature {
            name,
            generics: vec![],
            params,
            effects: vec![],
            return_type,
            where_clause: vec![],
        })
    }

    fn parse_import_decl(&mut self) -> Result<Decl, String> {
        self.next_token();
        let mut path = Vec::new();
        if let Token::Ident(p) = &self.current_token {
            path.push(p.clone());
        } else {
            return Err("Expected module path".into());
        }
        while self.peek_token == Token::Dot {
            self.next_token();
            if self.peek_token == Token::LBrace {
                break;
            }
            self.next_token();
            if let Token::Ident(p) = &self.current_token {
                path.push(p.clone());
            } else {
                return Err("Expected ident in import path".into());
            }
        }

        let mut items = Vec::new();
        let has_list = (self.current_token == Token::Dot && self.peek_token == Token::LBrace)
            || self.peek_token == Token::LBrace;
        if has_list {
            if self.current_token == Token::Dot {
                self.next_token();
            } else {
                self.next_token();
            }
            self.next_token();
            while self.current_token != Token::RBrace && self.current_token != Token::Eof {
                if let Token::Ident(n) = &self.current_token {
                    let name = n.clone();
                    let alias = if self.peek_token == Token::As {
                        self.next_token();
                        self.next_token();
                        if let Token::Ident(a) = &self.current_token { Some(a.clone()) } else { None }
                    } else { None };
                    items.push(ImportItem { name, alias });
                    if self.peek_token == Token::Comma {
                        self.next_token();
                        self.next_token();
                    } else { break; }
                } else { break; }
            }
            if !self.expect_peek(Token::RBrace) {
                return Err("Expected '}' in import list".into());
            }
        }

        if self.peek_token == Token::Semicolon {
            self.next_token();
        }
        self.next_token();

        Ok(Decl::Import { path, items })
    }

    pub fn parse_fn_decl(&mut self, is_pub: bool) -> Result<Decl, String> {
        self.parse_fn_decl_with_attrs(is_pub, vec![])
    }

    pub fn parse_fn_decl_with_attrs(
        &mut self,
        is_pub: bool,
        attrs: Vec<Attribute>,
    ) -> Result<Decl, String> {
        if let Token::Ident(_) = self.peek_token {
            self.next_token();
        } else {
            return Err("Expected function name".into());
        }

        let name = if let Token::Ident(n) = &self.current_token {
            n.clone()
        } else {
            return Err("Expected ident".into());
        };

        let generics = if self.peek_token == Token::Lt {
            self.next_token();
            self.parse_generic_params()?
        } else { vec![] };

        if !self.expect_peek(Token::LParen) {
            return Err("Expected '('".into());
        }
        let params = self.parse_params()?;

        let mut effects = Vec::new();
        let mut return_type = None;
        if self.peek_token == Token::Arrow {
            self.next_token();
            self.next_token();
            if self.current_token == Token::Lt {
                effects = self.parse_effect_set()?;
                self.next_token();
            }
            return_type = Some(self.parse_type()?);
        }

        let where_clause = if self.peek_token == Token::Where {
            self.next_token();
            self.parse_where_clause()?
        } else { vec![] };

        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{'".into());
        }
        let body = self.parse_block()?;

        Ok(Decl::Fn(FnDecl {
            attrs,
            is_pub,
            name,
            generics,
            params,
            effects,
            return_type,
            where_clause,
            body,
        }))
    }

    fn parse_where_clause(&mut self) -> Result<Vec<WhereBound>, String> {
        let mut bounds = Vec::new();
        loop {
            self.next_token();
            let ty = self.parse_type()?;
            if !self.expect_peek(Token::Colon) {
                return Err("Expected ':' in where bound".into());
            }
            self.next_token();
            let mut trait_paths = Vec::new();
            loop {
                let mut path = Vec::new();
                if let Token::Ident(n) = &self.current_token {
                    path.push(n.clone());
                } else {
                    return Err("Expected trait name in where bound".into());
                }
                while self.peek_token == Token::Dot {
                    self.next_token();
                    self.next_token();
                    if let Token::Ident(n) = &self.current_token {
                        path.push(n.clone());
                    } else {
                        return Err("Expected ident in qualified trait name".into());
                    }
                }
                trait_paths.push(path);
                if self.peek_token == Token::Plus {
                    self.next_token();
                    self.next_token();
                } else { break; }
            }
            bounds.push(WhereBound { ty, bounds: trait_paths });
            if self.peek_token == Token::Comma {
                self.next_token();
            } else { break; }
        }
        Ok(bounds)
    }

    pub fn parse_params(&mut self) -> Result<Vec<Param>, String> {
        let mut params = Vec::new();
        if self.peek_token == Token::RParen {
            self.next_token();
            return Ok(params);
        }
        self.next_token();

        loop {
            let span = self.current_span;
            let param = match &self.current_token {
                Token::SelfKW => Param::SelfVal,
                Token::Ampersand => {
                    self.next_token();
                    let is_mut = if self.current_token == Token::Mut { self.next_token(); true } else { false };
                    if self.current_token != Token::SelfKW {
                        return Err("Expected 'self' after '&'".into());
                    }
                    Param::SelfRef { is_mut }
                }
                Token::Ident(n) => {
                    let name = n.clone();
                    if !self.expect_peek(Token::Colon) {
                        return Err("Expected ':'".into());
                    }
                    self.next_token();
                    let ty = self.parse_type()?;
                    match (&name[..], &ty) {
                        ("self", Type::Reference { is_mut, .. }) => Param::SelfRef { is_mut: *is_mut },
                        ("self", _) => Param::SelfVal,
                        _ => Param::Named {
                            pattern: Spanned { node: Pattern::Bind(name), span },
                            ty,
                        },
                    }
                }
                _ => return Err(format!("Unexpected param token: {:?}", self.current_token)),
            };
            params.push(param);

            if self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
            } else {
                break;
            }
        }
        if !self.expect_peek(Token::RParen) {
            return Err("Expected ')'".into());
        }
        Ok(params)
    }
}
