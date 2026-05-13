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
            Token::Mod => {
                self.next_token();
                if let Token::Ident(name) = &self.current_token {
                    Ok(Decl::Mod(name.clone()))
                } else { Err("Expected module name".into()) }
            }
            _ => Err(format!("Unexpected declaration token: {:?}", self.current_token)),
        }
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
        match &self.current_token {
            Token::Ident(name) => Ok(Type::Named(name.clone())),
            Token::SelfUpper => Ok(Type::Named("Self".into())),
            Token::Ampersand => {
                self.next_token();
                let is_mut = if self.current_token == Token::Mut { self.next_token(); true } else { false };
                let inner = self.parse_type()?;
                Ok(Type::Reference { is_mut, inner: Box::new(inner), region: None })
            }
            _ => Err(format!("Unknown type: {:?}", self.current_token))
        }
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
            _ => return None,
        };
        Some(Spanned { node, span })
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