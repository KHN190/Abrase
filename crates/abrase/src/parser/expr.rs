use crate::ast::*;
use crate::lexer::Token;
use super::core::Parser;
use super::precedence::Precedence;
use super::helpers::is_block_terminated;

impl<'a> Parser<'a> {
    pub fn parse_expr(&mut self, precedence: Precedence) -> Spanned<Expr> {
        let span = self.current_span;
        let _guard = match self.enter_depth() {
            Some(g) => g,
            None => return Spanned { node: Expr::Error, span },
        };
        let mut left = match self.parse_prefix() {
            Some(expr) => expr,
            None => {
                self.report_error(format!("Unexpected token: {:?}", self.current_token), span);
                Spanned { node: Expr::Error, span }
            }
        };
        if is_block_terminated(&left.node)
            && !matches!(self.current_token,
                Token::Dot | Token::LParen | Token::LBracket | Token::Question)
        {
            return left;
        }
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
            Token::SelfKW => Expr::Identifier("self".into()),
            Token::LBracket => return Some(self.parse_array_literal(span)),
            Token::LParen => return Some(self.parse_paren_expr(span)),
            Token::Int(v) => Expr::Literal(Literal::Int(*v)),
            Token::Float(v) => Expr::Literal(Literal::Float(*v)),
            Token::String(v) => Expr::Literal(Literal::String(v.clone())),
            Token::StringInterp(parts) => Expr::Literal(Literal::StringInterp(parts.clone())),
            Token::Char(c) => Expr::Literal(Literal::Char(*c)),
            Token::True => Expr::Literal(Literal::Bool(true)),
            Token::False => Expr::Literal(Literal::Bool(false)),
            // `..end` or `..=end`.
            Token::Range | Token::RangeInclusive => {
                let inclusive = self.current_token == Token::RangeInclusive;
                if matches!(self.peek_token,
                    Token::Semicolon | Token::RBrace | Token::RParen | Token::RBracket
                        | Token::Comma | Token::LBrace | Token::Eof)
                {
                    return Some(Spanned {
                        node: Expr::Range { start: None, end: None, inclusive },
                        span,
                    });
                }
                self.next_token();
                let end = Box::new(self.parse_expr(Precedence::Range));
                return Some(Spanned {
                    node: Expr::Range { start: None, end: Some(end), inclusive },
                    span,
                });
            }
            Token::Bang | Token::Minus | Token::Ampersand | Token::Asterisk => {
                let op = match self.current_token {
                    Token::Bang => UnaryOp::Not,
                    Token::Minus => UnaryOp::Neg,
                    Token::Ampersand => {
                        // `&mut x` -> RefMut; plain `&x` -> Ref.
                        if self.peek_token == Token::Mut {
                            self.next_token();
                            UnaryOp::RefMut
                        } else {
                            UnaryOp::Ref
                        }
                    }
                    Token::Asterisk => UnaryOp::Deref,
                    _ => {
                        self.report_error(
                            format!("internal: prefix outer guard accepted {:?} but inner match did not",
                                    self.current_token),
                            span,
                        );
                        return None;
                    }
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
            Token::Pipe | Token::Or | Token::Move => { let r = self.parse_closure_expr(); return self.prefix_to_expr_or_err(r, span); }
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
                self.next_token();
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
        self.next_token();
        self.next_token();
        let mut fields = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let fname_span = self.current_span;
            let fname = if let Token::Ident(n) = &self.current_token { n.clone() } else {
                self.report_error(format!("Expected field name, got {:?}", self.current_token), self.current_span);
                break;
            };
            let (value, value_block_terminated) = if self.peek_token == Token::Colon {
                self.next_token();
                self.next_token();
                let v = self.parse_expr(Precedence::Lowest);
                let bt = is_block_terminated(&v.node);
                (Some(v), bt)
            } else {
                (None, false)
            };
            if !seen.insert(fname.clone()) {
                self.report_error(format!("Duplicate field '{}' in record literal", fname), fname_span);
            } else {
                fields.push(FieldInit { name: fname, value });
            }
            if value_block_terminated {
                if self.current_token == Token::Comma {
                    self.next_token();
                }
            } else if self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
            } else {
                self.next_token();
            }
        }
        // consume ending `}`.
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Spanned { node: Expr::Record { ty: vec![name], fields }, span }
    }

    fn parse_array_literal(&mut self, span: Span) -> Spanned<Expr> {
        self.next_token();
        if self.current_token == Token::RBracket {
            return Spanned { node: Expr::Array(vec![]), span };
        }
        let first = self.parse_expr(Precedence::Lowest);
        let first_block_terminated = is_block_terminated(&first.node);
        let sees_semi = self.peek_token == Token::Semicolon
            || (first_block_terminated && self.current_token == Token::Semicolon);
        if sees_semi {
            if self.current_token != Token::Semicolon { self.next_token(); }
            self.next_token();
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
        let mut items = vec![first];
        loop {
            if self.current_token == Token::Comma {
                self.next_token();
            } else if self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
            } else { break; }
            if self.current_token == Token::RBracket { break; }
            items.push(self.parse_expr(Precedence::Lowest));
        }
        if self.current_token == Token::RBracket {
            // already there (last item was block-terminated)
        } else if !self.expect_peek(Token::RBracket) {
            self.report_error("Expected ']' in array literal".into(), self.current_span);
        }
        Spanned { node: Expr::Array(items), span }
    }

    fn parse_if_expr(&mut self) -> Result<Expr, String> {
        self.next_token();
        let prev = self.no_record_literal;
        self.no_record_literal = true;
        let condition = Box::new(self.parse_expr(Precedence::Lowest));
        self.no_record_literal = prev;
        if self.current_token != Token::LBrace && !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' after if condition".into());
        }
        let consequence = Box::new(Spanned { node: Expr::Block(self.parse_block()?), span: self.current_span });
        let alternative = if self.current_token == Token::Else {
            self.next_token();
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
        self.next_token();
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
        self.next_token();
        let prev = self.no_record_literal;
        self.no_record_literal = true;
        let condition = Box::new(self.parse_expr(Precedence::Lowest));
        self.no_record_literal = prev;
        if self.current_token != Token::LBrace && !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in while loop".into());
        }
        let body = self.parse_block()?;
        Ok(Expr::While { condition, body })
    }

    fn parse_loop_expr(&mut self) -> Result<Expr, String> {
        if !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in loop".into());
        }
        let body = self.parse_block()?;
        Ok(Expr::Loop { body })
    }

    fn parse_closure_expr(&mut self) -> Result<Expr, String> {
        let is_move = if self.current_token == Token::Move {
            self.next_token();
            if self.current_token != Token::Pipe && self.current_token != Token::Or {
                return Err("Expected '|' or '||' after 'move' in closure".into());
            }
            true
        } else {
            false
        };
        let params: Vec<ClosureParam> = if self.current_token == Token::Or {
            Vec::new()
        } else {
            self.next_token();
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
            params
        };
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
        if self.peek_token == Token::RParen {
            self.next_token();
            return Spanned { node: Expr::Literal(Literal::Unit), span };
        }
        self.next_token();
        let first = self.parse_expr(Precedence::Lowest);
        let first_block_terminated = is_block_terminated(&first.node);
        if first_block_terminated {
            if self.current_token == Token::RParen {
                return Spanned { node: Expr::Paren(Box::new(first)), span };
            }
            if self.current_token == Token::Comma {
                let mut elems = vec![first];
                while self.current_token == Token::Comma {
                    self.next_token();
                    if self.current_token == Token::RParen { break; }
                    elems.push(self.parse_expr(Precedence::Lowest));
                    if !is_block_terminated(&elems.last().unwrap().node) {
                        if self.peek_token == Token::Comma || self.peek_token == Token::RParen {
                            self.next_token();
                        }
                    }
                }
                if self.current_token != Token::RParen {
                    self.report_error("Expected ')' in tuple expression".into(), self.current_span);
                    return Spanned { node: Expr::Error, span };
                }
                return Spanned { node: Expr::Tuple(elems), span };
            }
            self.report_error("Expected ')' in parenthesized expression".into(), self.current_span);
            return Spanned { node: Expr::Error, span };
        }
        if self.peek_token == Token::Comma {
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
        }
        Ok(Expr::Resume(arg))
    }

    fn parse_region_expr(&mut self) -> Result<Expr, String> {
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
        // match/handle prefix parsers exit one-past their own `}`. When the
        // body is one of them, current already sits at the outer `{`.
        if self.current_token != Token::LBrace && !self.expect_peek(Token::LBrace) {
            return Err("Expected '{' in handle".into());
        }
        let mut arms = Vec::new();
        self.next_token();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let kind = match &self.current_token {
                Token::Return => HandleArmKind::Return,
                Token::Throw => HandleArmKind::Exn,
                // BNF reserves `exn` as a handle-arm keyword. 
                Token::Ident(n) if n == "exn" => HandleArmKind::Exn,
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
            let pattern = if matches!(self.peek_token, Token::Ident(_) | Token::Underscore) {
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
                self.next_token();
            } else {
                break;
            }
        }
        // The loop exits with `current` either at the handle's closing `}`
        // or BEFORE it. Both must end with `current` ONE past `}`.
        if self.current_token != Token::RBrace && !self.expect_peek(Token::RBrace) {
            return Err("Expected '}' in handle".into());
        }
        self.next_token();
        Ok(Expr::Handle { expr, arms })
    }

    pub fn parse_infix(&mut self, left: Spanned<Expr>) -> Spanned<Expr> {
        let span = left.span;
        if matches!(self.current_token,
            Token::PlusAssign | Token::MinusAssign
            | Token::MulAssign | Token::DivAssign | Token::ModAssign)
        {
            let inner_op = match self.current_token {
                Token::PlusAssign  => BinaryOp::Add,
                Token::MinusAssign => BinaryOp::Sub,
                Token::MulAssign   => BinaryOp::Mul,
                Token::DivAssign   => BinaryOp::Div,
                Token::ModAssign   => BinaryOp::Mod,
                _ => unreachable!(),
            };
            let precedence = self.current_token.precedence();
            self.next_token();
            let right = self.parse_expr(precedence);
            let inner = Spanned {
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
                    right: Box::new(inner),
                },
                span,
            };
        }
        let node = match self.current_token {
            Token::Plus => BinaryOp::Add,
            Token::Minus => BinaryOp::Sub,
            Token::Asterisk => BinaryOp::Mul,
            Token::Slash => BinaryOp::Div,
            Token::Percent => BinaryOp::Mod,
            Token::Eq => BinaryOp::Eq,
            Token::NotEq => BinaryOp::Neq,
            Token::Lt => BinaryOp::Lt,
            Token::Gt => BinaryOp::Gt,
            Token::Lte => BinaryOp::Lte,
            Token::Gte => BinaryOp::Gte,
            Token::Or => BinaryOp::Or,
            Token::And => BinaryOp::And,
            Token::Ampersand => BinaryOp::BitAnd,
            Token::Pipe => BinaryOp::BitOr,
            Token::Caret => BinaryOp::BitXor,
            Token::Shl => BinaryOp::Shl,
            Token::Shr => BinaryOp::Shr,
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
                match &self.current_token {
                    Token::Ident(field) => {
                        return Spanned { node: Expr::FieldAccess { base: Box::new(left), field: field.clone() }, span: self.current_span };
                    }
                    Token::Int(n) if *n >= 0 => {
                        return Spanned { node: Expr::FieldAccess { base: Box::new(left), field: n.to_string() }, span: self.current_span };
                    }
                    _ => {
                        self.report_error("Expected field name or tuple index after dot".into(), self.current_span);
                        return Spanned { node: Expr::Error, span: self.current_span };
                    }
                }
            }
            Token::Question => {
                return Spanned { node: Expr::Question(Box::new(left)), span };
            }
            Token::Range | Token::RangeInclusive => {
                let inclusive = self.current_token == Token::RangeInclusive;
                let prec = self.current_token.precedence();
                let end = if matches!(
                    self.peek_token,
                    Token::Semicolon | Token::RBrace | Token::RParen | Token::RBracket
                        | Token::Comma | Token::LBrace | Token::Eof
                ) {
                    None
                } else {
                    self.next_token();
                    Some(Box::new(self.parse_expr(prec)))
                };
                return Spanned {
                    node: Expr::Range { start: Some(Box::new(left)), end, inclusive },
                    span,
                };
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
        let last_block = loop {
            let arg = self.parse_expr(Precedence::Lowest);
            let is_block = is_block_terminated(&arg.node);
            args.push(arg);
            if self.current_token == Token::Comma {
                self.next_token();
            } else if self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
            } else { break is_block; }
            if self.current_token == Token::RParen { break is_block; }
        };

        if !(last_block && self.current_token == Token::RParen) {
            self.expect_peek(Token::RParen);
        }
        Spanned { node: Expr::Call { callee: Box::new(callee), args }, span }
    }

    // Skip a run of `;` tokens. The lexer already eats whitespace.
    fn skip_semicolons(&mut self) {
        while self.current_token == Token::Semicolon {
            self.next_token();
        }
    }

    pub fn parse_block(&mut self) -> Result<Block, String> {
        let mut stmts = Vec::new();
        self.next_token();

        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            self.skip_semicolons();
            if self.current_token == Token::RBrace || self.current_token == Token::Eof {
                break;
            }
            let stmt_span = self.current_span;
            let was_block = match self.parse_stmt() {
                Ok(stmt) => {
                    let block = matches!(&stmt.node, Stmt::Expr(e) if is_block_terminated(&e.node));
                    stmts.push(stmt);
                    block
                }
                Err(msg) => {
                    self.report_error(msg, stmt_span);
                    self.synchronize();
                    continue;
                }
            };

            if !was_block && self.current_token != Token::Semicolon
                && self.current_token != Token::RBrace && self.current_token != Token::Eof {
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

            self.skip_semicolons();

            if self.current_token != Token::RBrace
                && self.current_token != Token::Eof
                && !Self::is_stmt_start(&self.current_token)
            {
                let tok = self.current_token.clone();
                self.report_error(
                    format!("Unexpected token {:?}; expected ';', '}}', or statement", tok.display()),
                    self.current_span,
                );
                self.synchronize();
            }
        }

        if self.current_token == Token::Eof {
            self.report_error("Expected '}' in block".into(), self.current_span);
        }
        // consume ending `}`.
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
