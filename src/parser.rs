use crate::ast::*;
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
            Token::Dot => Precedence::Index,
            _ => Precedence::Lowest,
        }
    }
}

pub struct ParseError {
    pub message: String,
    pub span: Span,
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
    current_span: Span,
    peek_token: Token,
    peek_span: Span,
    pub errors: Vec<ParseError>,
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
            errors: Vec::new() 
        }
    }

    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();
        self.current_span = self.peek_span;
        let (next_tok, next_span) = self.lexer.next_token();
        self.peek_token = next_tok;
        self.peek_span = next_span;
    }

    fn report_error(&mut self, message: String, span: Span) {
        self.errors.push(ParseError { message, span });
    }

    fn synchronize(&mut self) {
        self.next_token();
        while self.current_token != Token::Eof {
            if self.current_token == Token::Semicolon {
                self.next_token();
                return;
            }
            match self.peek_token {
                Token::Fn | Token::Let | Token::If | Token::Return => return,
                _ => self.next_token(),
            }
        }
    }

    fn expect_peek(&mut self, token: Token) -> bool {
        if self.peek_token == token {
            self.next_token();
            true
        } else {
            let msg = format!("Expected {:?}, got {:?}", token, self.peek_token);
            self.report_error(msg, self.peek_span); // Log error instead of crashing
            false
        }
    }

    pub fn parse_program(&mut self) -> Vec<Decl> {
        let mut decls = Vec::new();
        while self.current_token != Token::Eof {
            match self.parse_decl() {
                Ok(decl) => decls.push(decl),
                Err(_) => self.synchronize(), 
            }
            self.next_token();
        }
        decls
    }

    pub fn parse_decl(&mut self) -> Result<Decl, String> {
        let mut is_pub = false;
        let mut is_async = false;

        if self.current_token == Token::Pub {
            is_pub = true;
            self.next_token();
        }

        if self.current_token == Token::Async {
            is_async = true;
            self.next_token();
        }

        match self.current_token {
            Token::Fn => self.parse_fn_decl(is_pub, is_async),
            Token::Type => self.parse_type_decl(is_pub),
            Token::Trait => self.parse_trait_decl(is_pub),
            Token::Impl => self.parse_impl_decl(),
            Token::Const => self.parse_const_decl(is_pub),
            Token::Import => self.parse_import_decl(),
            Token::Mod => {
                self.next_token();
                if let Token::Ident(name) = &self.current_token {
                    Ok(Decl::Mod(name.clone()))
                } else { Err("Expected module name".into()) }
            }
            _ => Err(format!("Unexpected declaration token: {:?}", self.current_token)),
        }
    }

    fn parse_type_decl(&mut self, is_pub: bool) -> Result<Decl, String> {
        // current = 'type'
        self.next_token(); // type name
        let name = if let Token::Ident(n) = &self.current_token { n.clone() } else { return Err("Expected type name".into()); };

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
            TypeBody::Record(fields)
        } else {
            return Err("Expected type body (only record supported)".into());
        };

        Ok(Decl::Type { attrs: vec![], is_pub, ownership: None, name, generics: vec![], body })
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
                let fn_decl = self.parse_fn_decl(false, false)?;
                if let Decl::Fn(f) = fn_decl {
                    items.push(TraitItem::Default(f));
                }
            }
            if self.current_token != Token::RBrace {
                self.next_token();
            }
        }
        Ok(Decl::Trait { is_pub, name, generics: vec![], where_clause: vec![], items })
    }

    fn parse_impl_decl(&mut self) -> Result<Decl, String> {
        self.next_token();
        let for_type = self.parse_type()?;

        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in impl".into());
        }

        let mut methods = Vec::new();
        self.next_token();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            if self.current_token == Token::Fn {
                let fn_decl = self.parse_fn_decl(false, false)?;
                if let Decl::Fn(f) = fn_decl {
                    methods.push(f);
                }
            }
            if self.current_token != Token::RBrace {
                self.next_token();
            }
        }
        Ok(Decl::Impl { generics: vec![], trait_name: None, for_type, where_clause: vec![], methods })
    }

    fn parse_const_decl(&mut self, is_pub: bool) -> Result<Decl, String> {
        self.next_token();
        let name = if let Token::Ident(n) = &self.current_token { n.clone() } else { return Err("Expected const name".into()); };

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

        if self.peek_token == Token::Semicolon {
            self.next_token();
        }

        Ok(Decl::Const { is_pub, is_fn: false, name, generics: vec![], params: vec![], ty, value })
    }

    fn parse_import_decl(&mut self) -> Result<Decl, String> {
        self.next_token();
        let mut path = Vec::new();
        if let Token::Ident(p) = &self.current_token {
            path.push(p.clone());
        }
        while self.peek_token == Token::Dot {
            self.next_token();
            self.next_token();
            if let Token::Ident(p) = &self.current_token {
                path.push(p.clone());
            }
        }

        let mut items = Vec::new();
        if self.peek_token == Token::LBrace {
            self.next_token();
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

        Ok(Decl::Import { path, items })
    }

    pub fn parse_fn_decl(&mut self, is_pub: bool, is_async: bool) -> Result<Decl, String> {
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

        if !self.expect_peek(Token::LParen) {
            return Err("Expected '('".into());
        }
        let params = self.parse_params()?;

        let mut return_type = None;
        if self.peek_token == Token::Arrow {
            self.next_token();
            self.next_token();
            return_type = Some(self.parse_type()?);
        }

        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{'".into());
        }
        let body = self.parse_block()?;

        Ok(Decl::Fn(FnDecl {
            attrs: vec![],
            is_pub,
            is_async,
            name,
            generics: vec![],
            params,
            effects: vec![],
            return_type,
            where_clause: vec![],
            body,
        }))
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
                // dyn Trait
                if name == "dyn" {
                    self.next_token();
                    return match self.current_token.clone() {
                        Token::Ident(t) => Ok(Type::DynTrait(t)),
                        _ => Err("Expected trait name after 'dyn'".into()),
                    };
                }
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
                let pat_span = self.current_span;
                let name = if let Token::Ident(n) = &self.current_token { n.clone() } else { return Err("Expected let name".into()) };
                let pattern = Spanned { node: Pattern::Bind(name), span: pat_span };

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
            Token::Ident(name) => Expr::Identifier(name.clone()),
            Token::Int(v) => Expr::Literal(Literal::Int(*v)),
            Token::Float(v) => Expr::Literal(Literal::Float(*v)),
            Token::String(v) => Expr::Literal(Literal::String(v.clone())),
            Token::True => Expr::Literal(Literal::Bool(true)),
            Token::False => Expr::Literal(Literal::Bool(false)),
            Token::Bang | Token::Minus | Token::Ampersand => {
                let op = match self.current_token {
                    Token::Bang => UnaryOp::Not,
                    Token::Minus => UnaryOp::Neg,
                    Token::Ampersand => UnaryOp::Ref,
                    _ => unreachable!(),
                };
                self.next_token();
                let right = self.parse_expr(Precedence::Prefix);
                Expr::Unary { op, right: Box::new(right) }
            }
            Token::LBrace => self.parse_block_expr(),
            Token::If => return self.parse_if_expr().ok().map(|e| Spanned { node: e, span }),
            Token::Match => return self.parse_match_expr().ok().map(|e| Spanned { node: e, span }),
            Token::For => return self.parse_for_expr().ok().map(|e| Spanned { node: e, span }),
            Token::While => return self.parse_while_expr().ok().map(|e| Spanned { node: e, span }),
            Token::Loop => return self.parse_loop_expr().ok().map(|e| Spanned { node: e, span }),
            Token::Pipe => return self.parse_closure_expr().ok().map(|e| Spanned { node: e, span }),
            Token::Scope => return self.parse_scope_expr().ok().map(|e| Spanned { node: e, span }),
            Token::Region => return self.parse_region_expr().ok().map(|e| Spanned { node: e, span }),
            Token::Handle => return self.parse_handle_expr().ok().map(|e| Spanned { node: e, span }),
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

    fn parse_if_expr(&mut self) -> Result<Expr, String> {
        // current = 'if'
        self.next_token(); // move to condition
        let condition = Box::new(self.parse_expr(Precedence::Lowest));
        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' after if condition".into());
        }
        let consequence = Box::new(Spanned { node: Expr::Block(self.parse_block()?), span: self.current_span });
        let alternative = if self.peek_token == Token::Else {
            self.next_token(); // consume '}'
            self.next_token(); // move to 'if' or '{'
            if self.current_token == Token::If {
                Some(Box::new(Spanned { node: self.parse_if_expr()?, span: self.current_span }))
            } else if self.current_token == Token::LBrace {
                Some(Box::new(Spanned { node: Expr::Block(self.parse_block()?), span: self.current_span }))
            } else {
                return Err("Expected 'if' or '{' after 'else'".into());
            }
        } else { None };
        Ok(Expr::If { condition, consequence, alternative })
    }

    fn parse_match_expr(&mut self) -> Result<Expr, String> {
        self.next_token();
        let scrutinee = Box::new(self.parse_expr(Precedence::Lowest));
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
            arms.push(MatchArm { pattern, guard, body });
            if self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
            } else { break; }
        }
        if !self.expect_peek(Token::RBrace) {
            return Err("Expected '}' in match expr".into());
        }
        Ok(Expr::Match { scrutinee, arms })
    }

    fn parse_for_expr(&mut self) -> Result<Expr, String> {
        self.next_token();
        let pattern = self.parse_pattern()?;
        if !self.expect_peek(Token::In) {
            return Err("Expected 'in' in for loop".into());
        }
        self.next_token();
        let iter = Box::new(self.parse_expr(Precedence::Lowest));
        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in for loop".into());
        }
        let body = self.parse_block()?;
        Ok(Expr::For { pattern, iter, body })
    }

    fn parse_while_expr(&mut self) -> Result<Expr, String> {
        // current = 'while'
        self.next_token(); // move to condition
        let condition = Box::new(self.parse_expr(Precedence::Lowest));
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
        let ret_ty = if self.peek_token == Token::Arrow {
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
        Ok(Expr::Closure { is_move: false, params, effects: vec![], ret_ty, body })
    }

    fn parse_scope_expr(&mut self) -> Result<Expr, String> {
        // current = 'scope'
        let label = if self.peek_token == Token::Ident("_".into()) || matches!(self.peek_token, Token::Ident(_)) {
            self.next_token();
            match self.current_token.clone() {
                Token::Ident(n) => Some(n),
                _ => None,
            }
        } else { None };
        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in scope".into());
        }
        let body = self.parse_block()?;
        Ok(Expr::Scope { label, options: None, body })
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
        let expr = Box::new(self.parse_expr(Precedence::Lowest));
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
            arms.push(HandleArm { kind, pattern, body });
            if self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
            } else { break; }
        }
        if !self.expect_peek(Token::RBrace) {
            return Err("Expected '}' in handle".into());
        }
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
        let node = match self.current_token {
            Token::Plus => BinaryOp::Add,
            Token::Minus => BinaryOp::Sub,
            Token::Asterisk => BinaryOp::Mul,
            Token::Slash => BinaryOp::Div,
            Token::Eq => BinaryOp::Eq,
            Token::NotEq => BinaryOp::Neq,
            Token::Lt => BinaryOp::Lt,
            Token::Gt => BinaryOp::Gt,
            Token::Assign => BinaryOp::Assign,
            Token::LParen => return self.parse_call_expr(left),
            Token::Dot => {
                self.next_token();
                if self.current_token == Token::Await {
                    return Spanned { node: Expr::Await(Box::new(left)), span: self.current_span };
                } else if let Token::Ident(field) = &self.current_token {
                    return Spanned { node: Expr::FieldAccess { base: Box::new(left), field: field.clone() }, span: self.current_span };
                } else {
                    self.report_error("Expected field name after dot".into(), self.current_span);
                    return Spanned { node: Expr::Error, span: self.current_span };
                }
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
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(_) => {
                    self.synchronize();
                }
            }
            if self.current_token != Token::RBrace && self.current_token != Token::Eof {
                self.next_token();
            }
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