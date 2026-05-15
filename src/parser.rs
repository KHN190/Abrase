use crate::ast::*;
use crate::error::{Error, ErrorCode};
use crate::lexer::{Lexer, Token};

#[derive(PartialOrd, PartialEq, Clone, Copy)]
pub enum Precedence {
    Lowest,
    Assign,
    Or,
    And,
    Equals,
    LessGreater,
    Sum,
    Product,
    Prefix,
    Call,
    Index,
}

impl Token {
    fn precedence(&self) -> Precedence {
        match self {
            Token::Assign | Token::PlusAssign | Token::MinusAssign => Precedence::Assign,
            Token::Or => Precedence::Or,
            Token::And => Precedence::And,
            Token::Eq | Token::NotEq => Precedence::Equals,
            Token::Lt | Token::Gt | Token::Lte | Token::Gte => Precedence::LessGreater,
            Token::Plus | Token::Minus => Precedence::Sum,
            Token::Asterisk | Token::Slash | Token::Percent => Precedence::Product,
            Token::LParen => Precedence::Call,
            Token::LBracket => Precedence::Index,
            Token::Dot => Precedence::Index,
            Token::Question => Precedence::Index,
            _ => Precedence::Lowest,
        }
    }
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
    current_span: Span,
    peek_token: Token,
    peek_span: Span,
    pub errors: Vec<Error>,
    pub source: String,
    no_record_literal: bool,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let (cur_tok, cur_span) = lexer.next_token();
        let (peek_tok, peek_span) = lexer.next_token();
        Self {
            lexer,
            current_token: cur_tok,
            current_span: cur_span,
            peek_token: peek_tok,
            peek_span,
            errors: Vec::new(),
            source: String::new(),
            no_record_literal: false,
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = source;
        self
    }

    pub fn pretty_print_errors(&self) -> String {
        self.errors
            .iter()
            .map(|e| e.pretty_print(&self.source))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();
        self.current_span = self.peek_span;
        let (next_tok, next_span) = self.lexer.next_token();
        self.peek_token = next_tok;
        self.peek_span = next_span;
    }

    fn report_error(&mut self, message: String, span: Span) {
        self.errors.push(Error::new(ErrorCode::ParseError, span, message));
    }

    fn synchronize(&mut self) {
        self.next_token();
        while self.current_token != Token::Eof {
            if self.current_token == Token::Semicolon {
                self.next_token();
                return;
            }
            match self.peek_token {
                Token::Fn | Token::Let | Token::If | Token::Return
                | Token::Match | Token::While | Token::For | Token::Loop
                | Token::Type | Token::Trait | Token::Impl | Token::Const
                | Token::Import | Token::Effect | Token::Pub => return,
                _ => self.next_token(),
            }
        }
    }

    fn prefix_to_expr_or_err(&mut self, r: Result<Expr, String>, span: Span) -> Option<Spanned<Expr>> {
        match r {
            Ok(e) => Some(Spanned { node: e, span }),
            Err(msg) => {
                self.report_error(msg, span);
                Some(Spanned { node: Expr::Error, span })
            }
        }
    }

    fn is_stmt_start(tok: &Token) -> bool {
        matches!(tok,
            Token::Let | Token::Return | Token::If | Token::Match
            | Token::While | Token::For | Token::Loop | Token::Break
            | Token::Continue | Token::Throw | Token::LBrace
            | Token::Region | Token::Handle | Token::Resume
            | Token::Ident(_) | Token::Int(_) | Token::Float(_)
            | Token::String(_) | Token::True | Token::False
            | Token::Bang | Token::Minus | Token::Ampersand | Token::Asterisk | Token::LParen
        )
    }

    fn expect_peek(&mut self, token: Token) -> bool {
        if self.peek_token == token {
            self.next_token();
            true
        } else {
            let msg = format!("Expected {:?}, got {:?}", token, self.peek_token);
            self.report_error(msg, self.peek_span);
            false
        }
    }

    pub fn parse_program(&mut self) -> Vec<Decl> {
        let mut decls = Vec::new();
        while self.current_token != Token::Eof {
            match self.parse_decl() {
                Ok(decl) => {
                    decls.push(decl);
                    // Check for stray tokens after declaration
                    if self.current_token != Token::Eof &&
                       !matches!(self.current_token, Token::Fn | Token::Type | Token::Trait | Token::Impl | Token::Const | Token::Import | Token::Effect | Token::Mod | Token::Pub) {
                        self.report_error(format!("Unexpected token after declaration: {:?}", self.current_token), self.current_span);
                        self.synchronize();
                    }
                }
                Err(_) => self.synchronize(),
            }
        }
        decls
    }

    fn parse_attributes(&mut self) -> Result<Vec<Attribute>, String> {
        let mut attrs = Vec::new();
        while self.current_token == Token::At {
            self.next_token(); // past '@'
            let name = if let Token::Ident(n) = &self.current_token { n.clone() } else {
                return Err("Expected attribute name after '@'".into());
            };
            let mut args = Vec::new();
            if self.peek_token == Token::LParen {
                self.next_token(); // '('
                if self.peek_token != Token::RParen {
                    self.next_token(); // first arg
                    loop {
                        // Arg forms: <ident>, <literal>, or <ident> '=' <literal>.
                        let arg = match self.current_token.clone() {
                            Token::Ident(n) => {
                                if self.peek_token == Token::Assign {
                                    self.next_token(); // '='
                                    self.next_token(); // literal
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
            self.next_token(); // past last token of this attribute
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

    pub fn parse_decl(&mut self) -> Result<Decl, String> {
        // Optional leading attributes: '@name' or '@name(args...)'
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
                    self.next_token(); // '.'
                    self.next_token(); // ident
                    if let Token::Ident(p) = &self.current_token {
                        path.push(p.clone());
                    } else {
                        return Err("Expected ident in module path".into());
                    }
                }
                // Advance past the last ident
                self.next_token();
                Ok(Decl::Mod(path.join(".")))
            }
            _ => Err(format!("Unexpected declaration token: {:?}", self.current_token)),
        }
    }

    fn parse_type_decl_with_attrs(
        &mut self,
        is_pub: bool,
        attrs: Vec<Attribute>,
    ) -> Result<Decl, String> {
        // current is at the type name (parse_decl already advanced past 'type')
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

        // `impl Trait for Type { ... }` vs `impl Type { ... }`.
        let (trait_name, for_type) = if self.peek_token == Token::For {
            self.next_token(); // consume 'for'
            self.next_token(); // step onto the type
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
        // current = 'const'; may be followed by 'fn' for const-fn
        self.next_token();
        let is_fn = self.current_token == Token::Fn;
        if is_fn {
            self.next_token();
        }
        let name = if let Token::Ident(n) = &self.current_token { n.clone() } else { return Err("Expected const name".into()); };

        // Optional generic params: const fn name<T> ...
        let generics = if self.peek_token == Token::Lt {
            self.next_token();
            self.parse_generic_params()?
        } else { vec![] };

        // Optional fn-style param list: const fn name(...)
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
        // current = type name (after 'type' 'alias')
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
        // current = effect alias name (after 'effect' 'alias')
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
        // current = effect name (after 'effect')
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
        // Consume the closing '}'
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Ok(Decl::Effect { is_pub, name, ops })
    }

    fn parse_generic_params(&mut self) -> Result<Vec<GenericParam>, String> {
        // current = '<'
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
        // current = 'fn'
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
        // Path segments are '.<ident>'. Stop before '.{' (the import-list head).
        while self.peek_token == Token::Dot {
            self.next_token(); // current = '.'
            if self.peek_token == Token::LBrace {
                // '.{' — leave current on '.', the items branch consumes it.
                break;
            }
            self.next_token(); // ident
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
            // Advance to '{', then to first item or '}'.
            if self.current_token == Token::Dot {
                self.next_token(); // current = '{'
            } else {
                self.next_token(); // current = '{'
            }
            self.next_token(); // first ident or '}'
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

        // current is now the last token of the decl (last ident or '}').
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

        // Optional generic params: fn foo<T, U>(...)
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
            self.next_token(); // arrow
            self.next_token(); // thing after arrow
            // Check for effect set: -> <effect1, effect2> Type
            if self.current_token == Token::Lt {
                effects = self.parse_effect_set()?;
                self.next_token(); // advance past >
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
        // current = 'where'
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
                    // expect `self`
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
                    // `self: &Self` / `self: Self` / `self: &mut Self` → sugar for self params
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

    pub fn parse_type(&mut self) -> Result<Type, String> {
        match self.current_token.clone() {
            Token::Ident(name) => {
                // qualified path: a.b.c
                if self.peek_token == Token::Dot {
                    let mut path = vec![name];
                    while self.peek_token == Token::Dot {
                        self.next_token(); // '.'
                        self.next_token(); // next ident
                        match self.current_token.clone() {
                            Token::Ident(n) => path.push(n),
                            _ => return Err("Expected ident in qualified type".into()),
                        }
                    }
                    return Ok(Type::Qualified(path));
                }
                // generic: Name<T, U>
                if self.peek_token == Token::Lt {
                    self.next_token(); // '<'
                    let args = self.parse_type_args()?;
                    return Ok(Type::Generic { name, args });
                }
                Ok(Type::Named(name))
            }
            Token::SelfUpper => Ok(Type::Named("Self".into())),
            // &T, &mut T, &T in r, &mut T in r
            Token::Ampersand => {
                self.next_token();
                let is_mut = if self.current_token == Token::Mut {
                    self.next_token();
                    true
                } else { false };
                let inner = self.parse_type()?;
                let region = if self.peek_token == Token::In {
                    self.next_token(); // 'in'
                    self.next_token(); // region ident
                    match self.current_token.clone() {
                        Token::Ident(r) => Some(r),
                        _ => return Err("Expected region name after 'in'".into()),
                    }
                } else { None };
                Ok(Type::Reference { is_mut, inner: Box::new(inner), region })
            }
            // [T; N]
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
            // () | (T,) | (T, U) | (T) -> R | (T, U) -> <eff> R
            Token::LParen => {
                if self.peek_token == Token::RParen {
                    self.next_token(); // ')'
                    if self.peek_token == Token::Arrow {
                        self.next_token(); // '->'
                        let (effects, ret) = self.parse_fn_type_tail()?;
                        return Ok(Type::Function { params: vec![], effects, ret: Box::new(ret) });
                    }
                    return Ok(Type::Tuple(vec![]));
                }
                self.next_token();
                let mut types = vec![self.parse_type()?];
                while self.peek_token == Token::Comma {
                    self.next_token(); // ','
                    if self.peek_token == Token::RParen { break; } // trailing comma
                    self.next_token();
                    types.push(self.parse_type()?);
                }
                if !self.expect_peek(Token::RParen) {
                    return Err("Expected ')' in type".into());
                }
                if self.peek_token == Token::Arrow {
                    self.next_token(); // '->'
                    let (effects, ret) = self.parse_fn_type_tail()?;
                    return Ok(Type::Function { params: types, effects, ret: Box::new(ret) });
                }
                Ok(Type::Tuple(types))
            }
            _ => Err(format!("Unknown type token: {:?}", self.current_token)),
        }
    }

    fn parse_type_args(&mut self) -> Result<Vec<Type>, String> {
        if self.peek_token == Token::Gt {
            self.next_token();
            return Ok(vec![]);
        }
        self.next_token();
        let mut args = vec![self.parse_type()?];
        while self.peek_token == Token::Comma {
            self.next_token(); // ','
            if self.peek_token == Token::Gt { break; }
            self.next_token();
            args.push(self.parse_type()?);
        }
        if !self.expect_peek(Token::Gt) {
            return Err("Expected '>' in type args".into());
        }
        Ok(args)
    }

    fn parse_fn_type_tail(&mut self) -> Result<(Vec<EffectItem>, Type), String> {
        let effects = if self.peek_token == Token::Lt {
            self.next_token(); // '<'
            self.parse_effect_set()?
        } else { vec![] };
        self.next_token();
        let ret = self.parse_type()?;
        Ok((effects, ret))
    }

    fn parse_effect_set(&mut self) -> Result<Vec<EffectItem>, String> {
        if self.peek_token == Token::Gt {
            self.next_token();
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
                self.next_token(); // '.'
                self.next_token(); // ident
                match self.current_token.clone() {
                    Token::Ident(n) => name.push(n),
                    _ => return Err("Expected ident in effect name".into()),
                }
            }
            let arg = if self.peek_token == Token::Lt {
                self.next_token(); // '<'
                self.next_token(); // type
                let ty = self.parse_type()?;
                if !self.expect_peek(Token::Gt) {
                    return Err("Expected '>' after effect arg".into());
                }
                Some(Box::new(ty))
            } else { None };
            effects.push(EffectItem { name, arg });
            if self.peek_token == Token::Comma {
                self.next_token(); // ','
                self.next_token(); // next effect
            } else { break; }
        }
        if !self.expect_peek(Token::Gt) {
            return Err("Expected '>' after effect set".into());
        }
        Ok(effects)
    }

    pub fn parse_stmt(&mut self) -> Result<Spanned<Stmt>, String> {
        let span = self.current_span;
        match self.current_token {
            Token::Let => {
                self.next_token();
                let is_mut = if self.current_token == Token::Mut { self.next_token(); true } else { false };
                let pattern = self.parse_pattern()?;

                let mut ty = None;
                if self.peek_token == Token::Colon {
                    self.next_token();
                    self.next_token();
                    ty = Some(self.parse_type()?);
                }

                if !self.expect_peek(Token::Assign) {
                    return Err("Expected '='".into());
                }

                self.next_token();
                let value = self.parse_expr(Precedence::Lowest);

                if self.peek_token == Token::Semicolon {
                    self.next_token();
                }

                Ok(Spanned { node: Stmt::Let { pattern, is_mut, ty, value }, span })
            }
            _ => {
                let expr = self.parse_expr(Precedence::Lowest); // No '?' here
                if self.peek_token == Token::Semicolon { 
                    self.next_token(); 
                }

                Ok(Spanned { node: Stmt::Expr(expr), span })
            }
        }
    }

    pub fn parse_expr(&mut self, precedence: Precedence) -> Spanned<Expr> {
        let span = self.current_span;
        let mut left = match self.parse_prefix() {
            Some(expr) => expr,
            None => {
                self.report_error(format!("Unexpected token: {:?}", self.current_token), span);
                Spanned { node: Expr::Error, span }
            }
        };

        while self.peek_token != Token::Semicolon && precedence < self.peek_token.precedence() {
            self.next_token();
            left = self.parse_infix(left);
        }
        left
    }

    pub fn parse_prefix(&mut self) -> Option<Spanned<Expr>> {
        let span = self.current_span;
        let node = match &self.current_token {
            Token::Ident(name) => {
                let nm = name.clone();
                if !self.no_record_literal && self.peek_token == Token::LBrace {
                    return Some(self.parse_record_literal(nm, span));
                }
                Expr::Identifier(nm)
            }
            // Inside impl method bodies, `self` is the receiver binding.
            Token::SelfKW => Expr::Identifier("self".into()),
            Token::LBracket => return Some(self.parse_array_literal(span)),
            Token::LParen => return Some(self.parse_paren_expr(span)),
            Token::Int(v) => Expr::Literal(Literal::Int(*v)),
            Token::Float(v) => Expr::Literal(Literal::Float(*v)),
            Token::String(v) => Expr::Literal(Literal::String(v.clone())),
            Token::True => Expr::Literal(Literal::Bool(true)),
            Token::False => Expr::Literal(Literal::Bool(false)),
            Token::Bang | Token::Minus | Token::Ampersand | Token::Asterisk => {
                let op = match self.current_token {
                    Token::Bang => UnaryOp::Not,
                    Token::Minus => UnaryOp::Neg,
                    Token::Ampersand => UnaryOp::Ref,
                    Token::Asterisk => UnaryOp::Deref,
                    _ => unreachable!(),
                };
                self.next_token();
                let right = self.parse_expr(Precedence::Prefix);
                Expr::Unary { op, right: Box::new(right) }
            }
            Token::LBrace => self.parse_block_expr(),
            Token::If => { let r = self.parse_if_expr(); return self.prefix_to_expr_or_err(r, span); }
            Token::Match => { let r = self.parse_match_expr(); return self.prefix_to_expr_or_err(r, span); }
            Token::For => { let r = self.parse_for_expr(); return self.prefix_to_expr_or_err(r, span); }
            Token::While => { let r = self.parse_while_expr(); return self.prefix_to_expr_or_err(r, span); }
            Token::Loop => { let r = self.parse_loop_expr(); return self.prefix_to_expr_or_err(r, span); }
            Token::Pipe | Token::Move => { let r = self.parse_closure_expr(); return self.prefix_to_expr_or_err(r, span); }
            Token::Region => { let r = self.parse_region_expr(); return self.prefix_to_expr_or_err(r, span); }
            Token::Handle => { let r = self.parse_handle_expr(); return self.prefix_to_expr_or_err(r, span); }
            Token::Resume => { let r = self.parse_resume_expr(); return self.prefix_to_expr_or_err(r, span); }
            Token::Break => {
                self.next_token();
                let val = if self.current_token != Token::Semicolon && self.current_token != Token::RBrace && self.current_token != Token::Eof {
                    Some(Box::new(self.parse_expr(Precedence::Lowest)))
                } else { None };
                return Some(Spanned { node: Expr::Break(val), span });
            }
            Token::Continue => {
                return Some(Spanned { node: Expr::Continue, span });
            }
            Token::Return => {
                self.next_token();
                let val = if self.current_token != Token::Semicolon && self.current_token != Token::RBrace && self.current_token != Token::Eof {
                    Some(Box::new(self.parse_expr(Precedence::Lowest)))
                } else { None };
                return Some(Spanned { node: Expr::Return(val), span });
            }
            Token::Throw => {
                self.next_token();
                let expr = self.parse_expr(Precedence::Prefix);
                return Some(Spanned { node: Expr::Throw(Box::new(expr)), span });
            }
            _ => return None,
        };
        Some(Spanned { node, span })
    }

    fn parse_record_literal(&mut self, name: String, span: Span) -> Spanned<Expr> {
        self.next_token(); // to '{'
        self.next_token(); // to first field or '}'
        let mut fields = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let fname = if let Token::Ident(n) = &self.current_token { n.clone() } else {
                self.report_error(format!("Expected field name, got {:?}", self.current_token), self.current_span);
                break;
            };
            let value = if self.peek_token == Token::Colon {
                self.next_token();
                self.next_token();
                Some(self.parse_expr(Precedence::Lowest))
            } else {
                None
            };
            fields.push(FieldInit { name: fname, value });
            if self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
            } else {
                self.next_token();
            }
        }
        Spanned { node: Expr::Record { ty: vec![name], fields }, span }
    }

    fn parse_array_literal(&mut self, span: Span) -> Spanned<Expr> {
        self.next_token(); // past '['
        // Empty array
        if self.current_token == Token::RBracket {
            return Spanned { node: Expr::Array(vec![]), span };
        }
        let first = self.parse_expr(Precedence::Lowest);
        // Repeat form: [expr; count]
        if self.peek_token == Token::Semicolon {
            self.next_token(); // to ';'
            self.next_token(); // to count expr
            let count = self.parse_expr(Precedence::Lowest);
            if !self.expect_peek(Token::RBracket) {
                self.report_error("Expected ']' in array repeat".into(), self.current_span);
            }
            return Spanned {
                node: Expr::ArrayRepeat {
                    elem: Box::new(first),
                    count: Box::new(count),
                },
                span,
            };
        }
        // List form: [e0, e1, ...]
        let mut items = vec![first];
        while self.peek_token == Token::Comma {
            self.next_token(); // ','
            if self.peek_token == Token::RBracket { break; } // trailing comma
            self.next_token();
            items.push(self.parse_expr(Precedence::Lowest));
        }
        if !self.expect_peek(Token::RBracket) {
            self.report_error("Expected ']' in array literal".into(), self.current_span);
        }
        Spanned { node: Expr::Array(items), span }
    }

    fn parse_if_expr(&mut self) -> Result<Expr, String> {
        // current = 'if'
        self.next_token();
        let prev = self.no_record_literal;
        self.no_record_literal = true;
        let condition = Box::new(self.parse_expr(Precedence::Lowest));
        self.no_record_literal = prev;
        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' after if condition".into());
        }
        let consequence = Box::new(Spanned { node: Expr::Block(self.parse_block()?), span: self.current_span });
        let alternative = if self.current_token == Token::Else {
            self.next_token(); // move to 'if' or '{'
            if self.current_token == Token::If {
                Some(Box::new(Spanned { node: self.parse_if_expr()?, span: self.current_span }))
            } else if self.current_token == Token::LBrace {
                Some(Box::new(Spanned { node: Expr::Block(self.parse_block()?), span: self.current_span }))
            } else {
                return Err("Expected 'if' or '{' after 'else'".into());
            }
        } else { None };

        if alternative.is_some() && self.current_token == Token::Else {
            self.report_error("Unexpected else after terminal else".into(), self.current_span);
        } else if self.current_token != Token::RBrace &&
                  self.current_token != Token::Eof &&
                  self.current_token != Token::Semicolon &&
                  self.current_token != Token::Comma &&
                  self.current_token != Token::FatArrow &&
                  !matches!(self.current_token, Token::Else | Token::In) &&
                  matches!(self.current_token, Token::Ident(_)) {
            self.report_error("Unexpected token after if expression".into(), self.current_span);
        }

        Ok(Expr::If { condition, consequence, alternative })
    }

    fn parse_match_expr(&mut self) -> Result<Expr, String> {
        self.next_token();
        let prev = self.no_record_literal;
        self.no_record_literal = true;
        let scrutinee = Box::new(self.parse_expr(Precedence::Lowest));
        self.no_record_literal = prev;
        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' after match expr".into());
        }
        let mut arms = Vec::new();
        self.next_token();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let pattern = self.parse_pattern()?;
            let guard = if self.peek_token == Token::If {
                self.next_token();
                self.next_token();
                Some(self.parse_expr(Precedence::Lowest))
            } else { None };
            if !self.expect_peek(Token::FatArrow) {
                return Err("Expected '=>' in match arm".into());
            }
            self.next_token();
            let body = self.parse_expr(Precedence::Lowest);
            let body_is_block_terminated = is_block_terminated(&body.node);
            arms.push(MatchArm { pattern, guard, body });

            if body_is_block_terminated {
                if self.current_token == Token::Comma || self.current_token == Token::Semicolon {
                    self.next_token();
                }
            } else if self.peek_token == Token::Comma || self.peek_token == Token::Semicolon {
                self.next_token();
                self.next_token();
            } else {
                self.next_token();
            }
        }
        if self.current_token != Token::RBrace {
            return Err("Expected '}' in match expr".into());
        }
        self.next_token(); // consume '}'
        Ok(Expr::Match { scrutinee, arms })
    }

    fn parse_for_expr(&mut self) -> Result<Expr, String> {
        self.next_token();
        let pattern = self.parse_pattern()?;
        if !self.expect_peek(Token::In) {
            return Err("Expected 'in' in for loop".into());
        }
        self.next_token();
        let prev = self.no_record_literal;
        self.no_record_literal = true;
        let iter = Box::new(self.parse_expr(Precedence::Lowest));
        self.no_record_literal = prev;
        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in for loop".into());
        }
        let body = self.parse_block()?;
        Ok(Expr::For { pattern, iter, body })
    }

    fn parse_while_expr(&mut self) -> Result<Expr, String> {
        // current = 'while'
        self.next_token();
        let prev = self.no_record_literal;
        self.no_record_literal = true;
        let condition = Box::new(self.parse_expr(Precedence::Lowest));
        self.no_record_literal = prev;
        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in while loop".into());
        }
        let body = self.parse_block()?;
        Ok(Expr::While { condition, body })
    }

    fn parse_loop_expr(&mut self) -> Result<Expr, String> {
        // current = 'loop'
        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in loop".into());
        }
        let body = self.parse_block()?;
        Ok(Expr::Loop { body })
    }

    fn parse_closure_expr(&mut self) -> Result<Expr, String> {
        // current token is '|' for `|x| ...`, or 'move' for `move |x| ...`.
        let is_move = if self.current_token == Token::Move {
            self.next_token(); // past 'move' → '|'
            if self.current_token != Token::Pipe {
                return Err("Expected '|' after 'move' in closure".into());
            }
            true
        } else {
            false
        };
        self.next_token(); // past '|'
        let mut params = Vec::new();
        while self.current_token != Token::Pipe {
            let pat_span = self.current_span;
            let name = if let Token::Ident(n) = &self.current_token { n.clone() } else { return Err("Expected param name".into()); };
            let mut ty = None;
            if self.peek_token == Token::Colon {
                self.next_token();
                self.next_token();
                ty = Some(self.parse_type()?);
            }
            params.push(ClosureParam { pattern: Spanned { node: Pattern::Bind(name), span: pat_span }, ty });
            if self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
            } else { break; }
        }
        if !self.expect_peek(Token::Pipe) {
            return Err("Expected '|' after closure params".into());
        }
        let return_type = if self.peek_token == Token::Arrow {
            self.next_token();
            let (_, ret) = self.parse_fn_type_tail()?;
            Some(ret)
        } else { None };
        self.next_token();
        let body = Box::new(if self.current_token == Token::LBrace {
            Spanned { node: Expr::Block(self.parse_block()?), span: self.current_span }
        } else {
            self.parse_expr(Precedence::Lowest)
        });
        Ok(Expr::Closure { is_move, params, effects: vec![], return_type, body })
    }

    fn parse_paren_expr(&mut self, span: Span) -> Spanned<Expr> {
        // current = '('
        if self.peek_token == Token::RParen {
            // ()  → Unit literal
            self.next_token();
            return Spanned { node: Expr::Literal(Literal::Unit), span };
        }
        self.next_token();
        let first = self.parse_expr(Precedence::Lowest);
        if self.peek_token == Token::Comma {
            // (e1, e2, ...)
            let mut elems = vec![first];
            while self.peek_token == Token::Comma {
                self.next_token();
                if self.peek_token == Token::RParen { break; }
                self.next_token();
                elems.push(self.parse_expr(Precedence::Lowest));
            }
            if !self.expect_peek(Token::RParen) {
                self.report_error("Expected ')' in tuple expression".into(), self.current_span);
                return Spanned { node: Expr::Error, span };
            }
            return Spanned { node: Expr::Tuple(elems), span };
        }
        if !self.expect_peek(Token::RParen) {
            self.report_error("Expected ')' in parenthesized expression".into(), self.current_span);
            return Spanned { node: Expr::Error, span };
        }
        Spanned { node: first.node, span: first.span }
    }

    fn parse_resume_expr(&mut self) -> Result<Expr, String> {
        // current = 'resume'
        if !self.expect_peek(Token::LParen) {
            return Err("Expected '(' in resume".into());
        }
        self.next_token();
        let arg = if self.current_token == Token::RParen {
            None
        } else {
            let e = self.parse_expr(Precedence::Lowest);
            if !self.expect_peek(Token::RParen) {
                return Err("Expected ')' in resume".into());
            }
            Some(Box::new(e))
        };
        if arg.is_none() {
            // current is already RParen; nothing else to do
        }
        Ok(Expr::Resume(arg))
    }

    fn parse_region_expr(&mut self) -> Result<Expr, String> {
        // current = 'region'
        let label = if matches!(self.peek_token, Token::Ident(_)) {
            self.next_token();
            match self.current_token.clone() {
                Token::Ident(n) => Some(n),
                _ => None,
            }
        } else { None };
        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in region".into());
        }
        let body = self.parse_block()?;
        Ok(Expr::Region { label, body })
    }

    fn parse_handle_expr(&mut self) -> Result<Expr, String> {
        self.next_token();
        let prev = self.no_record_literal;
        self.no_record_literal = true;
        let expr = Box::new(self.parse_expr(Precedence::Lowest));
        self.no_record_literal = prev;
        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in handle".into());
        }
        let mut arms = Vec::new();
        self.next_token();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let kind = match &self.current_token {
                Token::Return => HandleArmKind::Return,
                Token::Throw => HandleArmKind::Exn,
                Token::Ident(n) => {
                    let mut path = vec![n.clone()];
                    while self.peek_token == Token::Dot {
                        self.next_token();
                        self.next_token();
                        if let Token::Ident(n) = &self.current_token { path.push(n.clone()); }
                    }
                    HandleArmKind::Effect(path)
                }
                _ => return Err("Expected 'return', 'exn', or effect name in handle arm".into()),
            };
            let pattern = if matches!(self.peek_token, Token::Ident(_)) {
                self.next_token();
                Some(self.parse_pattern()?)
            } else { None };
            if !self.expect_peek(Token::FatArrow) {
                return Err("Expected '=>' in handle arm".into());
            }
            self.next_token();
            let body = self.parse_expr(Precedence::Lowest);
            let body_is_block = is_block_terminated(&body.node);
            arms.push(HandleArm { kind, pattern, body });

            let sep_now_at_current = body_is_block
                && (self.current_token == Token::Comma
                    || self.current_token == Token::Semicolon);
            let sep_at_peek = !body_is_block
                && (self.peek_token == Token::Comma
                    || self.peek_token == Token::Semicolon);
            if sep_now_at_current {
                self.next_token();
            } else if sep_at_peek {
                self.next_token();
                self.next_token();
            } else if body_is_block && self.current_token != Token::RBrace {
                continue;
            } else if !body_is_block && self.peek_token != Token::RBrace {
                self.report_error(
                    format!("Expected ',' or '}}' in handle arms, got {:?}", self.peek_token),
                    self.peek_span,
                );
                self.next_token();
            } else {
                break;
            }
        }
        if !self.expect_peek(Token::RBrace) {
            return Err("Expected '}' in handle".into());
        }
        // Block-terminated exprs must leave current PAST their own '}' so the
        // enclosing parse_block does not consume it as its own closing brace.
        self.next_token();
        Ok(Expr::Handle { expr, arms })
    }

    pub fn parse_pattern(&mut self) -> Result<Spanned<Pattern>, String> {
        let span = self.current_span;
        let mut pats = vec![self.parse_pattern_primary()?];
        while self.peek_token == Token::Pipe {
            self.next_token(); // '|'
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

    fn parse_pattern_primary(&mut self) -> Result<Spanned<Pattern>, String> {
        let span = self.current_span;
        let node = match &self.current_token {
            Token::Underscore => Pattern::Wildcard,
            Token::Ampersand => {
                self.next_token();
                let pat = self.parse_pattern_primary()?;
                Pattern::Ref(Box::new(pat))
            }
            Token::Int(v) => {
                let start = Literal::Int(*v);
                if self.peek_token == Token::Range {
                    self.next_token(); // '..'
                    self.next_token();
                    let end = if let Token::Int(n) = self.current_token { Some(Literal::Int(n)) } else { None };
                    Pattern::Range { start: Some(start), end, inclusive: false }
                } else if self.peek_token == Token::RangeInclusive {
                    self.next_token(); // '..='
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
                if self.peek_token == Token::LParen || self.peek_token == Token::LBrace {
                    self.next_token(); // '(' or '{'
                    let (ty_path, args) = match self.current_token {
                        Token::LParen => {
                            let path = vec![name];
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
                            (path, args)
                        }
                        _ => (vec![name.clone()], vec![]),
                    };
                    Pattern::Variant { ty: ty_path, args }
                } else {
                    Pattern::Bind(name)
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

    pub fn parse_infix(&mut self, left: Spanned<Expr>) -> Spanned<Expr> {
        let span = left.span;
        // desugar 'a += b' / 'a -= b' to 
        // 'a = a + b' / 'a = a - b'. 
        if matches!(self.current_token, Token::PlusAssign | Token::MinusAssign) {
            let inner_op = if self.current_token == Token::PlusAssign {
                BinaryOp::Add
            } else {
                BinaryOp::Sub
            };
            let precedence = self.current_token.precedence();
            self.next_token();
            let right = self.parse_expr(precedence);
            let sum = Spanned {
                node: Expr::Binary {
                    op: inner_op,
                    left: Box::new(left.clone()),
                    right: Box::new(right),
                },
                span,
            };
            return Spanned {
                node: Expr::Binary {
                    op: BinaryOp::Assign,
                    left: Box::new(left),
                    right: Box::new(sum),
                },
                span,
            };
        }
        let node = match self.current_token {
            Token::Plus => BinaryOp::Add,
            Token::Minus => BinaryOp::Sub,
            Token::Asterisk => BinaryOp::Mul,
            Token::Slash => BinaryOp::Div,
            Token::Eq => BinaryOp::Eq,
            Token::NotEq => BinaryOp::Neq,
            Token::Lt => BinaryOp::Lt,
            Token::Gt => BinaryOp::Gt,
            Token::Lte => BinaryOp::Lte,
            Token::Gte => BinaryOp::Gte,
            Token::Assign => BinaryOp::Assign,
            Token::LParen => return self.parse_call_expr(left),
            Token::LBracket => {
                self.next_token();
                let index = self.parse_expr(Precedence::Lowest);
                let span = self.current_span;
                if !self.expect_peek(Token::RBracket) {
                    self.report_error("Expected ']' in index expression".into(), self.current_span);
                }
                return Spanned { node: Expr::Index { base: Box::new(left), index: Box::new(index) }, span };
            }
            Token::Dot => {
                self.next_token();
                if let Token::Ident(field) = &self.current_token {
                    return Spanned { node: Expr::FieldAccess { base: Box::new(left), field: field.clone() }, span: self.current_span };
                } else {
                    self.report_error("Expected field name after dot".into(), self.current_span);
                    return Spanned { node: Expr::Error, span: self.current_span };
                }
            }
            Token::Question => {
                return Spanned { node: Expr::Question(Box::new(left)), span };
            }
            _ => return Spanned { node: Expr::Error, span: self.current_span },
        };

        let precedence = self.current_token.precedence();
        self.next_token();
        let right = self.parse_expr(precedence);
        
        Spanned {
            node: Expr::Binary { 
                op: node, 
                left: Box::new(left), 
                right: Box::new(right) 
            },
            span,
        }
    }

    pub fn parse_call_expr(&mut self, callee: Spanned<Expr>) -> Spanned<Expr> {
        let span = callee.span;
        let mut args = Vec::new();
        if self.peek_token == Token::RParen {
            self.next_token();
            return Spanned { node: Expr::Call { callee: Box::new(callee), args }, span };
        }
        
        self.next_token();
        loop {
            args.push(self.parse_expr(Precedence::Lowest));
            if self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
            } else { break; }
        }
        
        self.expect_peek(Token::RParen);
        Spanned { node: Expr::Call { callee: Box::new(callee), args }, span }
    }

    pub fn parse_block(&mut self) -> Result<Block, String> {
        let mut stmts = Vec::new();
        self.next_token(); // consume '{'

        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let was_block = match self.parse_stmt() {
                Ok(stmt) => {
                    let block = matches!(&stmt.node, Stmt::Expr(e) if is_block_terminated(&e.node));
                    stmts.push(stmt);
                    block
                }
                Err(_) => { self.synchronize(); continue; }
            };

            if !was_block && self.current_token != Token::Semicolon {
                if self.peek_token == Token::Semicolon {
                    self.next_token();
                    self.next_token();
                } else if self.peek_token == Token::RBrace || self.peek_token == Token::Eof {
                    self.next_token();
                } else if Self::is_stmt_start(&self.peek_token) {
                    let peek_span = self.peek_span;
                    self.report_error(
                        "Expected ';' or newline between statements".into(),
                        peek_span,
                    );
                    self.next_token();
                } else {
                    self.next_token();
                }
            }

            while self.current_token == Token::Semicolon {
                self.next_token();
            }

            if self.current_token != Token::RBrace
                && self.current_token != Token::Eof
                && !Self::is_stmt_start(&self.current_token)
            {
                let tok = self.current_token.clone();
                self.report_error(
                    format!("Unexpected token {:?}; expected ';', '}}', or statement", tok),
                    self.current_span,
                );
                self.synchronize();
            }
        }

        if self.current_token == Token::Eof {
            self.report_error("Expected '}' in block".into(), self.current_span);
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }

        let mut ret = None;
        if let Some(spanned_stmt) = stmts.last() {
            if let Stmt::Expr(expr) = &spanned_stmt.node {
                ret = Some(Box::new(expr.clone()));
                stmts.pop();
            }
        }

        Ok(Block { stmts, ret })
    }

    pub fn parse_block_expr(&mut self) -> Expr {
        match self.parse_block() {
            Ok(block) => Expr::Block(block),
            Err(_) => Expr::Error,
        }
    }
}

fn is_block_terminated(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Block(_)
            | Expr::If { .. }
            | Expr::Match { .. }
            | Expr::While { .. }
            | Expr::For { .. }
            | Expr::Loop { .. }
            | Expr::Region { .. }
            | Expr::Handle { .. }
    )
}